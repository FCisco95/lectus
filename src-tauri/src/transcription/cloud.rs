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
        Self::with_model(base_url, api_key, "whisper-large-v3-turbo")
    }

    pub fn with_model(base_url: &str, api_key: &str, model: &str) -> Self {
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        Self {
            base_url: base_url.trim().trim_end_matches('/').to_string(),
            api_key: api_key.trim().to_string(),
            model: model.trim().to_string(),
            client,
        }
    }

    /// Transcribe 16kHz mono f32 samples by uploading them as a WAV file.
    /// Blocking — call from a blocking context (spawn_blocking), never an async task.
    ///
    /// `opts.language` of `None` omits the `language` field so the API
    /// auto-detects; `opts.initial_prompt` is sent as the `prompt` bias field.
    pub fn transcribe(&self, samples: &[f32], opts: &TranscribeOptions) -> Result<String> {
        let base = validate_provider_base(&self.base_url)?;
        if self.api_key.trim().is_empty() {
            return Err(anyhow!("cloud API key is not set"));
        }
        if self.model.is_empty() {
            return Err(anyhow!("choose a cloud transcription model in Settings"));
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

        let url = format!("{}/audio/transcriptions", base.trim_end_matches('/'));
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send().map_err(provider_transport_error)?;

        if !resp.status().is_success() {
            return Err(provider_status_error(resp.status()));
        }

        let parsed: TranscriptionResponse = serde_json::from_slice(&read_provider_response(resp)?)
            .map_err(|_| anyhow!("The provider returned an invalid transcription response. Check the speech endpoint and model."))?;
        Ok(parsed.text.trim().to_string())
    }
}

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// Read provider metadata without sending audio or text. A listed model is not
/// proof that inference is permitted for this account.
pub fn available_models(base_url: &str, api_key: &str) -> Result<Vec<String>> {
    if api_key.trim().is_empty() {
        return Err(anyhow!("Enter an API key before checking the connection."));
    }
    let base = validate_provider_base(base_url)?;
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build().map_err(|_| anyhow!("Could not initialize a secure provider connection."))?;
    let response = client.get(format!("{}/models", base.trim_end_matches('/')))
        .bearer_auth(api_key.trim()).send()
        .map_err(provider_transport_error)?;
    if !response.status().is_success() {
        return Err(provider_status_error(response.status()));
    }
    #[derive(Deserialize)]
    struct Model { id: String }
    #[derive(Deserialize)]
    struct ModelList { data: Vec<Model> }
    let bytes = read_provider_response(response)?;
    let list: ModelList = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow!("This endpoint did not return an OpenAI-compatible model list."))?;
    let mut ids: Vec<_> = list.data.into_iter().map(|m| m.id).collect();
    if ids.iter().any(|id| id.is_empty() || id.len() > 256 || id.chars().any(char::is_control) || id.contains(api_key.trim())) {
        return Err(anyhow!("The provider returned an invalid model list. Check the API base URL; no provider details were displayed."));
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

// Never include provider response bodies or reqwest errors here: either can
// contain echoed credentials, transcripts or URLs containing private data.
fn provider_transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!("The provider timed out. Check your connection and retry; no automatic retry was sent.")
    } else if error.is_connect() {
        anyhow!("Could not connect to the provider. Check the API base URL, DNS, firewall and TLS certificate.")
    } else {
        anyhow!("The provider connection was interrupted. Check your network and retry.")
    }
}

fn provider_status_error(status: reqwest::StatusCode) -> anyhow::Error {
    let recovery = match status.as_u16() {
        401 => "The provider rejected the API key. Check or replace the key in General settings.",
        403 => "The provider denied permission. Check your account and model access.",
        404 => "The provider endpoint was not found. Check the API base URL and selected model ID.",
        408 | 504 => "The provider timed out. Retry when the connection or service recovers.",
        413 => "The provider rejected the recording size. Use a shorter recording or check the provider limit.",
        429 => "The provider rate limit or quota was reached. Check usage and billing, wait, then retry.",
        500..=599 => "The provider is temporarily unavailable. Wait and retry; check its service status if this continues.",
        300..=399 => "The provider redirected the request. Enter its direct HTTPS API base URL; redirects are blocked to protect your key.",
        _ => "The provider rejected the request. Check the API base URL, model ID and account configuration.",
    };
    anyhow!("HTTP {}: {recovery}", status.as_u16())
}

fn read_provider_response(response: reqwest::blocking::Response) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    response.take(1_048_577).read_to_end(&mut bytes)
        .map_err(|_| anyhow!("The provider response was interrupted or timed out. Check the connection and retry."))?;
    if bytes.len() > 1_048_576 {
        return Err(anyhow!("The provider response is too large. Check that the URL points to an OpenAI-compatible API."));
    }
    Ok(bytes)
}

fn validate_provider_base(base_url: &str) -> Result<String> {
    let base = reqwest::Url::parse(base_url.trim())
        .map_err(|_| anyhow!("Enter a valid provider base URL."))?;
    let loopback = matches!(base.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if base.scheme() != "https" && !(base.scheme() == "http" && loopback) {
        return Err(anyhow!("Use HTTPS, or HTTP for a local provider."));
    }
    if !base.username().is_empty() || base.password().is_some() || base.query().is_some() || base.fragment().is_some() {
        return Err(anyhow!("Use the API base URL without credentials, query parameters or a fragment."));
    }
    Ok(base.to_string())
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
    fn provider_errors_explain_recovery_without_echoing_secrets() {
        for (status, expected) in [(401, "API key"), (403, "permission"), (404, "base URL"), (429, "quota"), (503, "temporarily")] {
            let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
            let url = format!("http://{}/v1", server.server_addr());
            let worker = std::thread::spawn(move || {
                let req = server.recv().unwrap();
                req.respond(tiny_http::Response::from_string("fixture-key private error body").with_status_code(status)).unwrap();
            });
            let error = available_models(&url, "fixture-key").unwrap_err().to_string();
            worker.join().unwrap();
            assert!(error.contains(expected), "HTTP {status}: {error}");
            assert!(!error.contains("fixture-key"));
            assert!(!error.contains("private error body"));
        }
    }

    #[test]
    fn transcription_failure_does_not_echo_provider_body_or_credentials() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", server.server_addr());
        let worker = std::thread::spawn(move || {
            let mut req = server.recv().unwrap();
            std::io::copy(req.as_reader(), &mut std::io::sink()).unwrap();
            req.respond(tiny_http::Response::from_string("fixture-key transcript private data").with_status_code(401)).unwrap();
        });
        let error = CloudWhisper::with_model(&url, "fixture-key", "speech")
            .transcribe(&[0.0; 16], &TranscribeOptions::default()).unwrap_err().to_string();
        worker.join().unwrap();
        assert!(error.contains("API key"));
        assert!(!error.contains("fixture-key"));
        assert!(!error.contains("private data"));
    }

    #[test]
    fn model_listing_rejects_oversized_and_secret_looking_ids() {
        for body in [format!("{{\"data\":[{{\"id\":\"fixture-key\"}}]}}"), "x".repeat(1_048_577)] {
            let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
            let url = format!("http://{}/v1", server.server_addr());
            let worker = std::thread::spawn(move || {
                let req = server.recv().unwrap();
                let _ = req.respond(tiny_http::Response::from_string(body));
            });
            let error = available_models(&url, "fixture-key").unwrap_err().to_string();
            worker.join().unwrap();
            assert!(!error.contains("fixture-key"));
        }
    }

    #[test]
    fn model_listing_redirect_does_not_forward_credentials() {
        let destination = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let redirect = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let target = format!("http://{}/stolen", destination.server_addr());
        let url = format!("http://{}/v1", redirect.server_addr());
        let worker = std::thread::spawn(move || {
            let req = redirect.recv().unwrap();
            req.respond(tiny_http::Response::empty(302).with_header(
                tiny_http::Header::from_bytes("Location", target).unwrap())).unwrap();
        });
        let error = available_models(&url, "fixture-key").unwrap_err().to_string();
        worker.join().unwrap();
        assert!(error.contains("redirect"));
        assert!(destination.recv_timeout(std::time::Duration::from_millis(50)).unwrap().is_none());
    }

    #[test]
    fn interrupted_model_list_does_not_echo_url_or_remote_body() {
        use std::io::{Read, Write};
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/private-path", server.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let (mut socket, _) = server.accept().unwrap();
            let mut request = [0u8; 4096];
            socket.read(&mut request).unwrap();
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4096\r\nConnection: close\r\n\r\nfixture-key private response").unwrap();
        });
        let error = available_models(&url, "fixture-key").unwrap_err().to_string();
        worker.join().unwrap();
        assert!(error.contains("interrupted"));
        assert!(!error.contains("fixture-key"));
        assert!(!error.contains("private-path"));
        assert!(!error.contains("private response"));
    }

    #[test]
    fn model_listing_checks_credentials_and_parses_provider_response() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", server.server_addr());
        let worker = std::thread::spawn(move || {
            let req = server.recv_timeout(std::time::Duration::from_secs(5)).unwrap().unwrap();
            assert_eq!(req.url(), "/v1/models");
            assert!(req.headers().iter().any(|h| h.field.equiv("Authorization") && h.value.as_str() == "Bearer fixture-key"));
            req.respond(tiny_http::Response::from_string(r#"{"data":[{"id":"speech"},{"id":"chat"},{"id":"speech"}]}"#)).unwrap();
        });
        assert_eq!(available_models(&url, " fixture-key ").unwrap(), vec!["chat", "speech"]);
        worker.join().unwrap();
        assert!(available_models("http://example.com/v1", "fixture-key").is_err());
        assert!(available_models(&url, "").is_err());
    }

    #[test]
    fn transcription_sends_selected_model_to_custom_endpoint() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", server.server_addr());
        let worker = std::thread::spawn(move || {
            let mut req = server.recv_timeout(std::time::Duration::from_secs(5)).unwrap().unwrap();
            assert_eq!(req.url(), "/v1/audio/transcriptions");
            let mut body = Vec::new();
            req.as_reader().read_to_end(&mut body).unwrap();
            assert!(body.windows(b"custom-speech-model".len()).any(|part| part == b"custom-speech-model"));
            req.respond(tiny_http::Response::from_string(r#"{"text":"Olá mundo"}"#)).unwrap();
        });
        let engine = CloudWhisper::with_model(&url, "fixture-key", "custom-speech-model");
        assert_eq!(engine.transcribe(&[0.0; 16], &TranscribeOptions::default()).unwrap(), "Olá mundo");
        worker.join().unwrap();
    }

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
