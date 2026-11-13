//! Optional real-model HTTP integration, isolated from installed settings and IO.
use std::sync::{Arc, Mutex};
use crate::{api, transcription};

struct FixtureBackend {
    engine: Arc<Mutex<transcription::local::LocalWhisper>>,
    worker: crate::worker::TranscribeWorker,
}
impl api::Backend for FixtureBackend {
    fn allowed(&self) -> bool { true }
    fn busy(&self) -> bool { false }
    fn transcribe(&self, samples: Vec<f32>, language: Option<String>, _cleanup: bool) -> Result<api::Transcript, api::ApiError> {
        let engine = self.engine.clone();
        self.worker.run(move || {
            let duration_ms = samples.len() as u64 * 1000 / 16_000;
            let start = std::time::Instant::now();
            let options = transcription::TranscribeOptions::from_config(language.as_deref().unwrap_or("auto"), None);
            let (text, language) = engine.lock().unwrap().transcribe(&samples, &options)
                .map_err(|_| api::ApiError::unavailable())?;
            Ok(api::Transcript { text, language, duration_ms, transcribe_ms: start.elapsed().as_millis() as u64, engine: "local".into() })
        })
    }
}

#[test]
#[ignore = "real Vulkan/Metal inference; requires downloaded Base model; no microphone or injection"]
fn real_local_api_transcribes_portuguese_fixture() {
    let model = transcription::model::bench_app_data_dir().join("models/ggml-base.bin");
    let backend = Arc::new(FixtureBackend {
        engine: Arc::new(Mutex::new(transcription::local::LocalWhisper::new(&model).expect("download Base before running"))),
        worker: crate::worker::TranscribeWorker::new(),
    });
    let directory = tempfile::tempdir().unwrap();
    let service = api::Service::new(directory.path().join("api.json"), backend, Arc::new(|_| {})).unwrap();
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    service.configure(api::Settings { enabled: true, port, allowed_origins: vec![] }).unwrap();
    let client = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(60)).build().unwrap();
    let base = format!("http://127.0.0.1:{port}");
    assert_eq!(client.get(format!("{base}/v1/health")).send().unwrap().status(), 401);
    let wav = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/audio_samples/tts_pt_6s.wav")).unwrap();
    let response = client.post(format!("{base}/v1/transcribe")).bearer_auth(service.token())
        .header("Content-Type", "audio/wav").header("X-Lectus-Language", "pt").body(wav).send().unwrap();
    assert_eq!(response.status(), 200);
    let result: serde_json::Value = response.json().unwrap();
    assert_eq!(result["engine"], "local");
    assert_eq!(result["language"], "pt");
    assert!(result["text"].as_str().unwrap().contains("quinta-feira"));
    assert!(result["duration_ms"].as_u64().unwrap() > 7000);
    println!("API real inference: audio={}ms processing={}ms, Portuguese weekday recognized", result["duration_ms"], result["transcribe_ms"]);
    service.stop();
    assert!(!service.status().running);
}
