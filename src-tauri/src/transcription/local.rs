use super::TranscribeOptions;
use anyhow::Result;
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

pub struct LocalWhisper {
    // Keep ctx alive: WhisperState holds a clone of the Arc internally, but
    // holding ctx here makes ownership intent explicit and avoids any edge-case
    // where the Arc could drop between state operations.
    #[allow(dead_code)]
    ctx: WhisperContext,
    // Pre-created once and reused across calls. Allocating the compute buffers
    // (KV caches + 4 scheduler graphs = ~200–400 MB for base) on every
    // dictation was the dominant latency contributor on Windows CPU-only builds.
    state: WhisperState,
}

impl LocalWhisper {
    pub fn new(model_path: &Path) -> Result<Self> {
        let mut params = WhisperContextParameters::default();
        // GPU inference — Vulkan on Windows, Metal on macOS (per-target Cargo
        // features); whisper.cpp falls back to CPU when no device is found.
        params.use_gpu(true);
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or_else(|| anyhow::anyhow!("invalid model path"))?,
            params,
        )?;
        let state = ctx.create_state()?;
        Ok(Self { ctx, state })
    }

    /// Transcribe 16kHz mono f32 samples. Returns the trimmed transcript and
    /// the language whisper used (detected or forced) as an ISO code — the
    /// cleanup LLM needs it to avoid translating instead of cleaning.
    ///
    /// `opts.language` of `None` lets whisper auto-detect (requires a
    /// multilingual model). `opts.initial_prompt` biases toward custom words.
    pub fn transcribe(
        &mut self,
        samples: &[f32],
        opts: &TranscribeOptions,
    ) -> Result<(String, Option<String>)> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        let n_threads = std::thread::available_parallelism()
            .map(|n| n.get().min(8))
            .unwrap_or(4) as i32;
        params.set_n_threads(n_threads);
        params.set_language(opts.language.as_deref());
        if let Some(prompt) = opts.initial_prompt.as_deref() {
            params.set_initial_prompt(prompt);
        }
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // Hallucination hardening. Each dictation is independent: carrying the
        // previous text as context lets one hallucinated phrase poison the next
        // output (whisper.cpp #2286). `initial_prompt` (dictionary bias) is
        // unaffected — it is injected regardless of no_context.
        params.set_no_context(true);
        // Tighter decoder-fallback thresholds than the 2.4 / -1.0 defaults:
        // reject low-confidence segments (typically silence/noise hallucinations).
        params.set_entropy_thold(2.6);
        params.set_logprob_thold(-1.25);
        // Drop non-speech token artifacts ("[BLANK_AUDIO]", music notes, etc.).
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);

        // Silero neural VAD (whisper.cpp built-in): gate frames before the
        // encoder. Trims leading/trailing silence (incl. the pre-roll ring)
        // and starves silence-triggered hallucinations.
        if let Some(vad_path) = opts.vad_model_path.as_deref() {
            params.set_vad_model_path(Some(vad_path));
            let mut vad = whisper_rs::WhisperVadParams::new();
            vad.set_threshold(0.6);
            vad.set_min_speech_duration(250); // ms — ignore blips
            vad.set_min_silence_duration(200); // ms — split point
            params.set_vad_params(vad);
            params.enable_vad(true);
        }

        self.state.full(params, samples)?;

        let n = self.state.full_n_segments();
        let text = (0..n)
            .filter_map(|i| self.state.get_segment(i))
            .filter_map(|seg| seg.to_str().ok().map(|s| s.to_string()))
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        let language = whisper_rs::get_lang_str(self.state.full_lang_id_from_state())
            .map(|s| s.to_string());

        Ok((text, language))
    }
}

/// Load a mono 16kHz WAV file as Vec<f32> in [-1.0, 1.0].
pub fn load_wav_as_f32(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| Ok(s?)).collect::<Result<Vec<_>>>()?,
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| Ok(s? as f32 / 32768.0))
            .collect::<Result<Vec<_>>>()?,
    };
    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Task A1 latency benchmark: per-model, per-clip wall-clock for short
    /// utterances (batch-1), plus model+state load time. Not a CI test.
    /// Run: cargo test --release bench_latency -- --ignored --nocapture
    /// (Vulkan build env required — see memory lectus-vulkan-build.)
    #[test]
    #[ignore = "benchmark — requires downloaded models in the app data dir"]
    fn bench_latency() {
        let model_dir = crate::transcription::model::bench_app_data_dir().join("models");
        let sample_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/audio_samples");

        let models = [
            "ggml-tiny.bin",
            "ggml-base.bin",
            "ggml-small.bin",
            "ggml-medium.bin",
            "ggml-large-v3-turbo.bin",
            "ggml-large-v3-turbo-q8_0.bin",
            "ggml-large-v3-turbo-q5_0.bin",
        ];
        let clips = ["tts_en_4s.wav", "jfk_en_11s.wav", "tts_pt_6s.wav"];

        println!("\n=== Lectus A1 latency bench ===");
        for model in models {
            let model_path = model_dir.join(model);
            if !model_path.exists() {
                println!("{model}: SKIP (not downloaded)");
                continue;
            }
            let t_load = std::time::Instant::now();
            let mut engine = LocalWhisper::new(&model_path).expect("model load failed");
            println!("\n{model}: load {} ms", t_load.elapsed().as_millis());

            // Second pass with Silero VAD when the model is present (A6).
            let vad_path = model_dir.join("ggml-silero-v5.1.2.bin");
            let vad_variants: Vec<Option<String>> = if vad_path.exists() {
                vec![None, vad_path.to_str().map(String::from)]
            } else {
                vec![None]
            };

            for clip in clips {
                let wav = sample_dir.join(clip);
                let samples = load_wav_as_f32(&wav).expect("wav load failed");
                let audio_secs = samples.len() as f32 / 16000.0;

                for vad in &vad_variants {
                    let opts = TranscribeOptions {
                        vad_model_path: vad.clone(),
                        ..Default::default()
                    };
                    // Warmup run — mirrors the app's GPU warmup at engine load.
                    let _ = engine.transcribe(&samples, &opts).expect("warmup failed");

                    let mut times = Vec::new();
                    let mut text = String::new();
                    for _ in 0..3 {
                        let t = std::time::Instant::now();
                        text = engine.transcribe(&samples, &opts).expect("transcribe failed").0;
                        times.push(t.elapsed().as_millis());
                    }
                    let best = *times.iter().min().unwrap();
                    println!(
                        "  {clip} ({audio_secs:.1}s audio, vad={}): runs {:?} ms | best {} ms | RTF {:.3} | \"{}\"",
                        vad.is_some(),
                        times,
                        best,
                        best as f32 / 1000.0 / audio_secs,
                        text
                    );
                }
            }
        }
        println!("\n=== end bench ===\n");
    }

    #[test]
    #[ignore = "requires model file — run: cargo test -- --ignored after download_model"]
    fn test_transcribe_known_audio() {
        let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("models/ggml-tiny.en.bin");
        assert!(
            model_path.exists(),
            "model not found at {:?} — run scripts/download_model.ps1",
            model_path
        );

        let mut engine = LocalWhisper::new(&model_path).expect("failed to load model");

        let wav_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tests/audio_samples/hello_world_16k.wav");

        if !wav_path.exists() {
            eprintln!("Skipping: test audio not found at {:?}", wav_path);
            return;
        }

        let samples = load_wav_as_f32(&wav_path).expect("failed to load wav");
        let (result, _lang) = engine
            .transcribe(&samples, &TranscribeOptions::default())
            .expect("transcription failed");
        println!("Transcript: {result:?}");
        assert!(!result.is_empty(), "transcript should not be empty");
    }
}
