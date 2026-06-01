use anyhow::Result;
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct LocalWhisper {
    ctx: WhisperContext,
}

impl LocalWhisper {
    pub fn new(model_path: &Path) -> Result<Self> {
        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or_else(|| anyhow::anyhow!("invalid model path"))?,
            params,
        )?;
        Ok(Self { ctx })
    }

    /// Transcribe 16kHz mono f32 samples. Returns trimmed transcript string.
    pub fn transcribe(&self, samples: &[f32]) -> Result<String> {
        let mut state = self.ctx.create_state()?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state.full(params, samples)?;

        let n = state.full_n_segments();
        let text = (0..n)
            .filter_map(|i| state.get_segment(i))
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

        let engine = LocalWhisper::new(&model_path).expect("failed to load model");

        // Load test WAV
        let wav_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tests/audio_samples/hello_world_16k.wav");

        if !wav_path.exists() {
            eprintln!("Skipping: test audio not found at {:?}", wav_path);
            return;
        }

        let samples = load_wav_as_f32(&wav_path).expect("failed to load wav");
        let result = engine.transcribe(&samples).expect("transcription failed");
        println!("Transcript: {result:?}");
        assert!(!result.is_empty(), "transcript should not be empty");
    }
}
