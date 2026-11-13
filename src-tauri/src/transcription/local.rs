use super::TranscribeOptions;
use anyhow::Result;
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

pub struct LocalWhisper {
    // Keep ctx alive: WhisperState holds a clone of the Arc internally, but
    // holding ctx here makes ownership intent explicit and avoids any edge-case
    // where the Arc could drop between state operations. Also used to count
    // dictionary prompt tokens.
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
        // With a word list, rebuild the prompt using the model's own tokenizer so
        // the 223-token limit in whisper.cpp never cuts words silently.
        let rebuilt;
        let prompt: Option<&str> = if opts.vocabulary.is_empty() {
            opts.initial_prompt.as_deref()
        } else {
            let ctx = &self.ctx;
            rebuilt = super::dictionary::bias_prompt_within(
                &opts.vocabulary,
                super::dictionary::PROMPT_TOKEN_BUDGET,
                // A text can never need more tokens than bytes, so `len + 16`
                // cannot trip whisper_tokenize's negative "too small" return.
                // A tokenizer failure counts as over budget.
                |s| ctx.tokenize(s, s.len() + 16).map(|t| t.len()).unwrap_or(usize::MAX),
            );
            match &rebuilt {
                Some((p, left_out)) => {
                    if *left_out > 0 {
                        log::warn!("dictionary prompt over token budget: {left_out} word(s) left out");
                    }
                    Some(p.as_str())
                }
                None => {
                    log::warn!("dictionary prompt over token budget: no words fit, prompt skipped");
                    None
                }
            }
        };
        if let Some(prompt) = prompt {
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

    fn owner_fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/audio_samples/owner")
    }

    /// Records 6 s from the default microphone into the git-ignored owner
    /// fixture folder. Run via `scripts\record-fixture.cmd <name>`, e.g.
    /// `pt_mycel_1` (the `pt_` / `en_` prefix selects the language later).
    #[test]
    #[ignore = "records from the microphone — run scripts/record-fixture.cmd <name>"]
    fn record_owner_fixture() {
        let name = std::env::var("LECTUS_FIXTURE_NAME").expect("set LECTUS_FIXTURE_NAME");
        assert!(
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
            "fixture name may only use letters, digits, _ and -"
        );
        let capture = crate::audio::AudioCapture::start(None).expect("microphone unavailable");
        println!("Recording 6 seconds. Speak now.");
        std::thread::sleep(std::time::Duration::from_secs(6));
        let samples = capture.drain();
        drop(capture);

        let dir = owner_fixture_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.wav"));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for s in &samples {
            writer.write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16).unwrap();
        }
        writer.finalize().unwrap();
        println!("Wrote {} ({:.1}s)", path.display(), samples.len() as f32 / 16_000.0);
    }

    /// Compares five prompt variants on every owner fixture and prints the
    /// transcript and prompt token count for each. Reads only the
    /// `dictionary_words` field of the installed app's config.json.
    /// Run: cargo test --release --lib bench_vocabulary_bias -- --ignored --nocapture
    #[test]
    #[ignore = "benchmark — needs owner fixtures and ggml-large-v3-turbo.bin in the app data dir"]
    fn bench_vocabulary_bias() {
        let dir = owner_fixture_dir();
        let mut wavs: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path())
                .filter(|p| p.extension().map(|x| x == "wav").unwrap_or(false)).collect())
            .unwrap_or_default();
        wavs.sort();
        if wavs.is_empty() {
            println!("SKIP: no WAV files in {}. Record some with scripts/record-fixture.cmd", dir.display());
            return;
        }
        let app_dir = crate::transcription::model::bench_app_data_dir();
        let model_path = app_dir.join("models").join("ggml-large-v3-turbo.bin");
        assert!(model_path.exists(), "model missing: {}", model_path.display());
        let words: Vec<String> = std::fs::read_to_string(app_dir.join("config.json")).ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("dictionary_words").cloned())
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        let target = std::env::var("LECTUS_BENCH_TARGET").unwrap_or_else(|_| "Mycel".into());
        let mut engine = LocalWhisper::new(&model_path).expect("model load failed");

        let mut target_last: Vec<String> = words.iter().filter(|w| !w.eq_ignore_ascii_case(&target)).cloned().collect();
        target_last.push(target.clone());
        let joined = |list: &[String]| list.join(", ");

        println!("\n=== vocabulary bias bench: {} words, target {target} ===", words.len());
        for wav in &wavs {
            let stem = wav.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let lang = if stem.starts_with("pt_") { "pt" } else if stem.starts_with("en_") { "en" } else { "" };
            let sentence = if lang == "pt" {
                format!("Neste texto falamos de {}.", joined(&words))
            } else {
                format!("This text mentions {}.", joined(&words))
            };
            let variants: Vec<(&str, Option<String>)> = vec![
                ("no prompt", None),
                ("current prompt", crate::transcription::dictionary::bias_prompt(&words)),
                ("target last", crate::transcription::dictionary::bias_prompt(&target_last)),
                ("sentence", Some(sentence)),
                ("target alone", Some(format!("Vocabulary: {target}."))),
            ];
            let samples = load_wav_as_f32(wav).expect("wav load failed");
            println!("\n{stem} (language {})", if lang.is_empty() { "auto" } else { lang });
            for (label, prompt) in variants {
                let tokens = prompt.as_deref()
                    .map(|p| engine.ctx.tokenize(p, p.len() + 16).map(|t| t.len()).unwrap_or(0))
                    .unwrap_or(0);
                let opts = TranscribeOptions {
                    language: if lang.is_empty() { None } else { Some(lang.to_string()) },
                    initial_prompt: prompt,
                    ..Default::default()
                };
                let (text, _) = engine.transcribe(&samples, &opts).expect("transcribe failed");
                println!("  {label:<15} tokens {tokens:>3} | {text}");
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
