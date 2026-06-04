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
        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or_else(|| anyhow::anyhow!("invalid model path"))?,
            params,
        )?;
        let state = ctx.create_state()?;
        Ok(Self { ctx, state })
    }

    /// Transcribe 16kHz mono f32 samples. Returns trimmed transcript string.
    ///
    /// `opts.language` of `None` lets whisper auto-detect (requires a
    /// multilingual model). `opts.initial_prompt` biases toward custom words.
    pub fn transcribe(&mut self, samples: &[f32], opts: &TranscribeOptions) -> Result<String> {
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

        self.state.full(params, samples)?;

        let n = self.state.full_n_segments();
        let text = (0..n)
            .filter_map(|i| self.state.get_segment(i))
            .filter_map(|seg| seg.to_str().ok().map(|s| s.to_string()))
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        Ok(text)
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
        let result = engine
            .transcribe(&samples, &TranscribeOptions::default())
            .expect("transcription failed");
        println!("Transcript: {result:?}");
        assert!(!result.is_empty(), "transcript should not be empty");
    }
}
