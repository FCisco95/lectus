use super::*;
use std::{io::{Read, Write}, net::TcpStream, sync::Condvar};

struct MockBackend {
    allowed: AtomicBool,
    busy: AtomicBool,
    ready: AtomicBool,
    calls: AtomicUsize,
    block: Mutex<bool>,
    release: Condvar,
}
impl MockBackend {
    fn new() -> Self {
        Self { allowed: AtomicBool::new(true), busy: AtomicBool::new(false), ready: AtomicBool::new(true),
            calls: AtomicUsize::new(0), block: Mutex::new(false), release: Condvar::new() }
    }
}
impl Backend for MockBackend {
    fn allowed(&self) -> bool { self.allowed.load(Ordering::SeqCst) }
    fn busy(&self) -> bool { self.busy.load(Ordering::SeqCst) }
    fn ready(&self) -> bool { self.ready.load(Ordering::SeqCst) }
    fn transcribe(&self, samples: Vec<f32>, language: Option<String>, cleanup: bool) -> Result<Transcript, ApiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let blocked = self.block.lock().unwrap();
        let (blocked, timeout) = self.release.wait_timeout_while(blocked, Duration::from_secs(4), |b| *b).unwrap();
        assert!(!*blocked && !timeout.timed_out(), "test backend was not released");
        Ok(Transcript { text: if cleanup { "Clean private phrase" } else { "Private phrase" }.into(), language,
            duration_ms: samples.len() as u64 / 16, transcribe_ms: 1, engine: "local".into() })
    }
}

fn port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}
fn fixture() -> (tempfile::TempDir, Arc<MockBackend>, Arc<Service>) {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MockBackend::new());
    let service = Arc::new(Service::new(dir.path().join("api.json"), backend.clone(), Arc::new(|_| {})).unwrap());
    service.configure(Settings { enabled: true, port: port(), allowed_origins: vec!["https://organic.example".into()] }).unwrap();
    (dir, backend, service)
}
fn read_response(socket: TcpStream) -> String {
    read_response_with_prefix(socket, Vec::new())
}
fn read_response_with_prefix(mut socket: TcpStream, mut response: Vec<u8>) -> String {
    socket.set_read_timeout(Some(Duration::from_secs(7))).unwrap();
    if let Err(error) = socket.read_to_end(&mut response) {
        assert!(error.kind() == std::io::ErrorKind::ConnectionReset && !response.is_empty(), "read failed: {error}");
    }
    let response = String::from_utf8(response).unwrap();
    let (headers, body) = response.split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("Incomplete HTTP response: {response:?}"));
    let expected = headers.lines().find_map(|line| line.strip_prefix("Content-Length: "))
        .unwrap_or_else(|| panic!("Missing response length: {response:?}")).parse::<usize>().unwrap();
    assert_eq!(body.len(), expected, "Truncated HTTP response: {response:?}");
    response
}
fn request(service: &Service, method: &str, path: &str, extra: &str, body: &[u8]) -> String {
    let port = service.status().port;
    let mut socket = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
    write!(socket, "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{extra}Content-Length: {}\r\n\r\n", body.len()).unwrap();
    socket.write_all(body).unwrap();
    read_response(socket)
}
fn auth(service: &Service) -> String { format!("Authorization: Bearer {}\r\n", service.token()) }
fn speech(service: &Service, body: &[u8]) -> String {
    request(service, "POST", "/v1/transcribe", &format!("{}Content-Type: audio/wav\r\n", auth(service)), body)
}
fn wait_until(check: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !check() {
        assert!(Instant::now() < deadline, "condition did not become true");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn disabled_start_does_not_generate_credentials_or_write_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("api.json");
    let service = Service::new(path.clone(), Arc::new(MockBackend::new()), Arc::new(|_| {})).unwrap();
    service.start().unwrap();
    assert!(!path.exists());
    assert!(service.token().is_empty());
    assert!(!service.status().enabled && !service.status().running);
}

#[test]
fn authenticated_wav_roundtrip_has_cors_and_metadata_but_no_transcript_in_status() {
    let (_dir, backend, service) = fixture();
    let response = request(&service, "POST", "/v1/transcribe", &format!(
        "{}Origin: https://organic.example\r\nContent-Type: audio/wav\r\nX-Lectus-Language: pt\r\nX-Lectus-Cleanup: 1\r\n", auth(&service)), &audio::fixture(16_000, 1, 160));
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains("Access-Control-Allow-Origin: https://organic.example"));
    assert!(!response.contains("Allow-Credentials"));
    assert!(response.contains("Clean private phrase") && response.contains("\"language\":\"pt\""));
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
    let status = service.status();
    assert_eq!(status.calls.len(), 1);
    assert_eq!(status.calls[0].duration_ms, Some(10));
    assert_eq!(status.calls[0].result_chars, Some(20));
    let serialized = serde_json::to_string(&status).unwrap();
    assert!(!serialized.contains("private phrase") && !serialized.contains(&service.token()));
}

#[test]
fn host_origin_and_auth_are_checked_before_decoding_or_inference() {
    let (_dir, backend, service) = fixture();
    for extra in [String::new(), "Authorization: Bearer invalid\r\n".into()] {
        assert!(request(&service, "POST", "/v1/transcribe", &extra, &[]).contains("401"));
    }
    for origin in ["http://organic.example", "https://sub.organic.example", "https://organic.example:443",
        "https://organic.example/", "https://Organic.example", "null"]
    {
        let result = request(&service, "POST", "/v1/transcribe", &format!("{}Origin: {origin}\r\n", auth(&service)), &[]);
        assert!(result.contains("403") && !result.contains("Access-Control-Allow-Origin:"));
    }
    let port = service.status().port;
    let mut socket = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
    write!(socket, "POST /v1/transcribe HTTP/1.1\r\nHost: evil.example:{port}\r\n{}Content-Length: 0\r\n\r\n", auth(&service)).unwrap();
    assert!(read_response(socket).contains("forbidden_host"));
    assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn preflight_needs_exact_origin_method_and_headers_but_not_bearer() {
    let (_dir, _backend, service) = fixture();
    let good = "Origin: https://organic.example\r\nAccess-Control-Request-Method: POST\r\nAccess-Control-Request-Headers: authorization, content-type, x-lectus-language\r\n";
    let response = request(&service, "OPTIONS", "/v1/transcribe", good, &[]);
    assert!(response.starts_with("HTTP/1.1 204"));
    assert!(response.contains("Access-Control-Allow-Headers:"));
    assert!(!response.contains("Allow-Credentials"));
    for invalid in [good.replace("POST", "DELETE"), good.replace("x-lectus-language", "cookie"), good.replace("https://organic.example", "https://other.example")] {
        assert!(!request(&service, "OPTIONS", "/v1/transcribe", &invalid, &[]).starts_with("HTTP/1.1 204"));
    }
    let settings = service.inner.state.lock().unwrap().stored.settings.clone();
    service.configure(Settings { allowed_origins: vec![], ..settings }).unwrap();
    assert!(request(&service, "OPTIONS", "/v1/transcribe", good, &[]).contains("forbidden_origin"));
}

#[test]
fn refuses_media_shapes_oversize_lengths_and_unsupported_headers() {
    let (_dir, backend, service) = fixture();
    assert!(request(&service, "POST", "/v1/transcribe", &auth(&service), &[]).contains("415"));
    for body in [b"invalid".to_vec(), audio::fixture(44_100, 1, 16), audio::fixture(16_000, 2, 16)] {
        assert!(speech(&service, &body).contains("400"));
    }
    let port = service.status().port;
    let mut socket = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
    write!(socket, "POST /v1/transcribe HTTP/1.1\r\nHost: localhost:{port}\r\n{}Content-Type: audio/wav\r\nContent-Length: {}\r\n\r\n", auth(&service), audio::MAX_BYTES + 1).unwrap();
    assert!(read_response(socket).contains("413"));
    let malformed = request(&service, "POST", "/v1/transcribe", &format!("{}Transfer-Encoding: chunked\r\n", auth(&service)), &[]);
    assert!(malformed.contains("400"));
    let language = request(&service, "POST", "/v1/transcribe", &format!("{}Content-Type: audio/wav\r\nX-Lectus-Language: ../../bad\r\n", auth(&service)), &audio::fixture(16_000, 1, 16));
    assert!(language.contains("bad_language"), "unexpected response: {language:?}");
    assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn membership_readiness_and_dictation_busy_are_enforced_per_call() {
    let (_dir, backend, service) = fixture();
    backend.allowed.store(false, Ordering::SeqCst);
    assert!(request(&service, "GET", "/v1/health", &auth(&service), &[]).contains("locked"));
    let locked = speech(&service, &audio::fixture(16_000, 1, 16));
    assert!(locked.contains("locked"), "unexpected response: {locked:?}");
    backend.allowed.store(true, Ordering::SeqCst);
    backend.ready.store(false, Ordering::SeqCst);
    assert!(request(&service, "GET", "/v1/health", &auth(&service), &[]).contains("engine_unavailable"));
    backend.ready.store(true, Ordering::SeqCst);
    backend.busy.store(true, Ordering::SeqCst);
    assert!(speech(&service, &audio::fixture(16_000, 1, 16)).contains("busy"));
    assert!(request(&service, "GET", "/v1/health", &auth(&service), &[]).contains("\"busy\":true"));
    assert!(!service.status().busy, "dictation must not pulse the API activity indicator");
    assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn early_rejection_response_survives_an_upload_arriving_during_close() {
    let (_dir, backend, service) = fixture();
    backend.allowed.store(false, Ordering::SeqCst);
    let body = vec![0u8; 64 * 1024];
    // Deliberately begin the upload after the error response starts. The old
    // immediate close could reset TCP and discard its remaining headers/body.
    for round in 0..20 {
        let port = service.status().port;
        let mut socket = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        socket.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
        write!(socket, "POST /v1/transcribe HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{}Content-Type: audio/wav\r\nContent-Length: {}\r\n\r\n", auth(&service), body.len()).unwrap();
        let mut prefix = vec![0u8; 16];
        socket.read_exact(&mut prefix).unwrap();
        let upload = socket.write_all(&body);
        let response = read_response_with_prefix(socket, prefix);
        assert!(response.starts_with("HTTP/1.1 403") && response.contains("locked"),
            "round {round}, upload={upload:?}, response={response:?}");
    }
    assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn concurrent_requests_reject_instead_of_queuing_inference() {
    let (_dir, backend, service) = fixture();
    *backend.block.lock().unwrap() = true;
    let sender = service.clone();
    let first = std::thread::spawn(move || speech(&sender, &audio::fixture(16_000, 1, 160)));
    wait_until(|| backend.calls.load(Ordering::SeqCst) == 1);
    assert!(service.status().busy);
    assert!(speech(&service, &audio::fixture(16_000, 1, 160)).contains("busy"));
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
    *backend.block.lock().unwrap() = false;
    backend.release.notify_all();
    assert!(first.join().unwrap().starts_with("HTTP/1.1 200"));
    assert!(!service.status().busy);
}

#[test]
fn rotation_and_disable_during_upload_revoke_old_authority() {
    for disable in [false, true] {
        let (_dir, backend, service) = fixture();
        let body = audio::fixture(16_000, 1, 160);
        let port = service.status().port;
        let mut socket = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
        write!(socket, "POST /v1/transcribe HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{}Content-Type: audio/wav\r\nContent-Length: {}\r\n\r\n", auth(&service), body.len()).unwrap();
        wait_until(|| service.inner.busy.load(Ordering::SeqCst));
        assert!(!service.status().busy, "upload is not inference activity");
        if disable {
            service.configure(Settings { enabled: false, port, allowed_origins: vec![] }).unwrap();
        } else { service.regenerate_token().unwrap(); }
        socket.write_all(&body).unwrap();
        assert!(read_response(socket).contains("unauthorized"));
        assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn rate_limit_and_call_log_are_bounded() {
    let (_dir, backend, service) = fixture();
    for _ in 0..30 {
        assert!(request(&service, "POST", "/v1/transcribe", &auth(&service), &[]).contains("415"));
    }
    assert!(request(&service, "POST", "/v1/transcribe", &auth(&service), &[]).contains("429"));
    let calls = service.status().calls;
    assert_eq!(calls.len(), 5);
    assert!(calls.windows(2).all(|pair| pair[0].timestamp_ms <= pair[1].timestamp_ms));
    assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn failed_reconfiguration_preserves_the_working_listener_and_credentials() {
    let (_dir, _backend, service) = fixture();
    let before = service.status();
    let token = service.token();
    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    assert!(service.configure(Settings { enabled: true, port: occupied.local_addr().unwrap().port(), allowed_origins: vec![] }).is_err());
    assert_eq!(service.status().port, before.port);
    assert!(service.status().running && service.status().error.is_some());
    assert_eq!(service.token(), token);
    assert!(request(&service, "GET", "/v1/health", &auth(&service), &[]).starts_with("HTTP/1.1 200"));
}

#[test]
fn failed_persistence_does_not_enable_or_change_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let blocked = dir.path().join("blocked");
    std::fs::write(&blocked, "fixture").unwrap();
    let service = Service::new(blocked.join("api.json"), Arc::new(MockBackend::new()), Arc::new(|_| {})).unwrap();
    assert!(service.configure(Settings { enabled: true, port: port(), allowed_origins: vec![] }).is_err());
    assert!(!service.status().enabled && !service.status().running);
    assert!(service.token().is_empty());
    assert!(service.regenerate_token().is_err());
    assert!(service.token().is_empty());
}

#[test]
fn persisted_settings_reopen_with_same_token_and_bind_only_on_start() {
    let (dir, _backend, service) = fixture();
    let token = service.token();
    let port = service.status().port;
    service.stop();
    drop(service);
    let restored = Service::new(dir.path().join("api.json"), Arc::new(MockBackend::new()), Arc::new(|_| {})).unwrap();
    assert!(restored.status().enabled && !restored.status().running);
    assert_eq!(restored.token(), token);
    restored.start().unwrap();
    assert!(restored.status().running);
    assert_eq!(restored.status().port, port);
    assert!(request(&restored, "GET", "/v1/health", &auth(&restored), &[]).starts_with("HTTP/1.1 200"));
}

#[test]
fn invalid_ports_and_non_origin_allowlist_entries_are_rejected() {
    assert!(validate_settings(&Settings { port: 0, ..Settings::default() }).is_err());
    for origin in ["*", "null", "https://*.example", "https://example/path", "https://example/", "https://user:pass@example"] {
        assert!(validate_settings(&Settings { allowed_origins: vec![origin.into()], ..Settings::default() }).is_err(), "{origin}");
    }
}

#[test]
fn health_is_rate_limited_and_slow_clients_cannot_spawn_unbounded_workers() {
    let (_dir, _backend, service) = fixture();
    let sockets: Vec<_> = (0..4).map(|_| TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, service.status().port)).unwrap()).collect();
    wait_until(|| service.inner.clients.load(Ordering::SeqCst) == 4);
    let mut extra = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, service.status().port)).unwrap();
    extra.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut byte = [0u8; 1];
    let result = extra.read(&mut byte);
    assert!(matches!(result, Ok(0)) || result.is_err_and(|e| e.kind() == std::io::ErrorKind::ConnectionReset));
    assert!(service.inner.clients.load(Ordering::SeqCst) <= 4);
    drop(sockets);
    wait_until(|| service.inner.clients.load(Ordering::SeqCst) == 0);
    for _ in 0..30 {
        assert!(request(&service, "GET", "/v1/health", &auth(&service), &[]).starts_with("HTTP/1.1 200"));
    }
    assert!(request(&service, "GET", "/v1/health", &auth(&service), &[]).contains("429"));
}
