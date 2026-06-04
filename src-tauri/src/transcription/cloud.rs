use super::TranscribeOptions;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::io::Cursor;

/// OpenAI-compatible cloud transcription backend (Groq by default).
pub struct CloudWhisper {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

impl CloudWhisper {
    /// Build a cloud backend. `base_url` is the API root (no trailing
    /// `/audio/transcriptions`), e.g. "https://api.groq.com/openai/v1".
    pub fn new(base_url: &str, api_key: &str) -> Self {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: "whisper-large-v3-turbo".to_string(),
            client,
        }
    }

    /// Transcribe 16kHz mono f32 samples by uploading them as a WAV file.
    /// Blocking — call from a blocking context (spawn_blocking), never an async task.
    ///
    /// `opts.language` of `None` omits the `language` field so the API
    /// auto-detects; `opts.initial_prompt` is sent as the `prompt` bias field.
    pub fn transcribe(&self, samples: &[f32], opts: &TranscribeOptions) -> Result<String> {
        if self.api_key.trim().is_empty() {
            return Err(anyhow!("cloud API key is not set"));
        }
        let wav = samples_to_wav_bytes(samples)?;

        let part = reqwest::blocking::multipart::Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;
        let mut form = reqwest::blocking::multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone())
            .text("response_format", "json");
        if let Some(lang) = opts.language.as_deref() {
            form = form.text("language", lang.to_string());
        }
        if let Some(prompt) = opts.initial_prompt.as_deref() {
            form = form.text("prompt", prompt.to_string());
        }

        let url = format!("{}/audio/transcriptions", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(anyhow!("cloud transcription failed ({status}): {body}"));
        }

        let parsed: TranscriptionResponse = resp.json()?;
        Ok(parsed.text.trim().to_string())
    }
}

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// Encode 16kHz mono f32 samples (in [-1.0, 1.0]) to a 16-bit PCM WAV in memory.
pub fn samples_to_wav_bytes(samples: &[f32]) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::<u8>::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
        for &s in samples {
            let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            writer.write_sample(v)?;
        }
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_samples_to_wav_bytes_has_riff_header() {
        let samples = vec![0.0f32, 0.5, -0.5, 1.0, -1.0];
        let bytes = samples_to_wav_bytes(&samples).expect("encode failed");
        // WAV files start with the ASCII tag "RIFF"
        assert_eq!(&bytes[0..4], b"RIFF", "expected RIFF header");
        // "WAVE" format tag at offset 8
        assert_eq!(&bytes[8..12], b"WAVE", "expected WAVE tag");
        // Non-trivial payload (header is 44 bytes; 5 samples * 2 bytes = 10 more)
        assert!(bytes.len() >= 44 + 10, "wav too short: {}", bytes.len());
    }

    #[test]
    fn test_samples_to_wav_bytes_roundtrips_via_hound() {
        // Encode, then decode with hound — sample count must survive.
        let samples: Vec<f32> = (0..1600).map(|i| ((i as f32) * 0.01).sin() * 0.3).collect();
        let bytes = samples_to_wav_bytes(&samples).expect("encode failed");
        let mut reader = hound::WavReader::new(std::io::Cursor::new(bytes)).expect("read failed");
        let decoded: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
        assert_eq!(decoded.len(), samples.len(), "sample count changed on roundtrip");
    }

    #[test]
    fn test_samples_to_wav_bytes_boundary_values() {
        // 0.0 → 0, 1.0 → 32767, -1.0 → -32767 (clamp then scale by 32767.0)
        let bytes = samples_to_wav_bytes(&[0.0, 1.0, -1.0]).expect("encode failed");
        let mut reader = hound::WavReader::new(std::io::Cursor::new(bytes)).expect("read failed");
        let decoded: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
        assert_eq!(decoded, vec![0i16, 32767, -32767]);
    }

    #[test]
    #[ignore = "requires GROQ_API_KEY env var and network — run: cargo test cloud -- --ignored"]
    fn test_cloud_transcribe_live() {
        let key = std::env::var("GROQ_API_KEY").expect("set GROQ_API_KEY to run this test");
        let engine = CloudWhisper::new("https://api.groq.com/openai/v1", &key);
        // 1s of 440Hz tone — Groq returns *some* text; we only assert no error.
        let samples: Vec<f32> = (0..16000).map(|i| ((i as f32) * 0.1).sin() * 0.2).collect();
        let result = engine.transcribe(&samples, &TranscribeOptions::default());
        assert!(result.is_ok(), "cloud transcription errored: {:?}", result);
    }
}
