use super::{audio, token, ApiError, Inner, Transcript};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{atomic::{AtomicBool, Ordering}, Arc, Weak},
    time::{Duration, Instant},
};

const HEADER_LIMIT: usize = 16 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CLIENTS: usize = 4;
const CLOSE_TIMEOUT: Duration = Duration::from_millis(250);

pub(super) fn serve(listener: TcpListener, inner: Weak<Inner>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, address)) => {
                if !address.ip().is_loopback() { continue; }
                let Some(inner) = inner.upgrade() else { break };
                // The single accept loop is the only incrementer. Workers only
                // decrement, so this cannot oversubscribe the bounded pool.
                if inner.clients.load(Ordering::SeqCst) >= MAX_CLIENTS { continue; }
                inner.clients.fetch_add(1, Ordering::SeqCst);
                std::thread::spawn(move || {
                    struct Client(Arc<Inner>);
                    impl Drop for Client { fn drop(&mut self) { self.0.clients.fetch_sub(1, Ordering::SeqCst); } }
                    let _client = Client(inner.clone());
                    handle(stream, &inner);
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => {
                if let Some(inner) = inner.upgrade() {
                    inner.running.store(false, Ordering::SeqCst);
                    inner.state.lock().unwrap().error = Some("The local API listener stopped. Save API settings to retry.".into());
                    inner.notify();
                }
                break;
            }
        }
    }
}

struct Request {
    method: String,
    path: String,
    headers: HashMap<String, String>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> { self.headers.get(name).map(String::as_str) }
}

fn timeout() -> ApiError { ApiError::new(408, "request_timeout", "The local API request timed out.") }
fn malformed() -> ApiError { ApiError::new(400, "bad_request", "Malformed HTTP request.") }
fn unauthorized() -> ApiError { ApiError::new(401, "unauthorized", "A valid API bearer token is required.") }

fn read_chunk(stream: &mut TcpStream, bytes: &mut [u8], deadline: Instant) -> Result<usize, ApiError> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() { return Err(timeout()); }
        stream.set_read_timeout(Some(remaining.min(Duration::from_secs(1)))).map_err(|_| malformed())?;
        match stream.read(bytes) {
            Ok(0) => return Err(malformed()),
            Ok(count) => return Ok(count),
            Err(error) if matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted) => {},
            Err(_) => return Err(malformed()),
        }
    }
}

fn read_request(stream: &mut TcpStream, deadline: Instant) -> Result<Request, ApiError> {
    let mut bytes = Vec::with_capacity(1024);
    let mut byte = [0u8; 1];
    while !bytes.ends_with(b"\r\n\r\n") {
        if bytes.len() >= HEADER_LIMIT { return Err(ApiError::new(431, "headers_too_large", "Request headers are too large.")); }
        read_chunk(stream, &mut byte, deadline)?;
        bytes.push(byte[0]);
    }
    if !bytes.is_ascii() { return Err(malformed()); }
    let text = std::str::from_utf8(&bytes).map_err(|_| malformed())?;
    let mut lines = text[..text.len() - 4].split("\r\n");
    let parts: Vec<_> = lines.next().ok_or_else(malformed)?.split(' ').collect();
    if parts.len() != 3 || !matches!(parts[2], "HTTP/1.1" | "HTTP/1.0") { return Err(malformed()); }
    let mut headers = HashMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or_else(malformed)?;
        if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || value.bytes().any(|c| (c < 32 && c != b'\t') || c == 127)
        { return Err(malformed()); }
        if headers.insert(name.to_ascii_lowercase(), value.trim().to_string()).is_some() { return Err(malformed()); }
    }
    if headers.contains_key("transfer-encoding") { return Err(malformed()); }
    Ok(Request { method: parts[0].into(), path: parts[1].into(), headers })
}

fn handle(mut stream: TcpStream, inner: &Arc<Inner>) {
    // Accepted Windows sockets can inherit the listener's nonblocking mode.
    if stream.set_nonblocking(false).is_err() { return; }
    let deadline = Instant::now() + REQUEST_TIMEOUT;
    let mut cors = None;
    let result = read_request(&mut stream, deadline).and_then(|request| {
        let (settings, expected_token, generation) = {
            let state = inner.state.lock().unwrap();
            (state.stored.settings.clone(), state.stored.token.clone(), state.generation)
        };
        if !inner.current(generation) { return Err(unauthorized()); }
        let host = request.header("host").ok_or_else(malformed)?;
        if host != format!("127.0.0.1:{}", settings.port) && host != format!("localhost:{}", settings.port) {
            return Err(ApiError::new(403, "forbidden_host", "The request Host must match this loopback API port."));
        }
        if let Some(origin) = request.header("origin") {
            if !settings.allowed_origins.iter().any(|allowed| allowed == origin) {
                return Err(ApiError::new(403, "forbidden_origin", "This origin is not allowed."));
            }
            cors = Some(origin.to_string());
        }
        if request.method == "OPTIONS" { return preflight(&request, cors.as_deref()); }
        if !token::matches(request.header("authorization"), &expected_token) { return Err(unauthorized()); }
        if !inner.backend.allowed() { return Err(ApiError::locked()); }
        if !inner.rate_allowed() { return Err(ApiError::new(429, "rate_limited", "At most 30 authenticated requests per minute are allowed.")); }
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/v1/health") => {
                let busy = inner.busy.load(Ordering::SeqCst) || inner.backend.busy();
                if !busy && !inner.backend.ready() { return Err(ApiError::unavailable()); }
                Ok(Response::json(200, serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"busy":busy})))
            }
            ("POST", "/v1/transcribe") => {
                if inner.backend.busy() || inner.busy.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                    return Err(ApiError::busy());
                }
                struct Busy<'a>(&'a Inner);
                impl Drop for Busy<'_> {
                    fn drop(&mut self) { self.0.busy.store(false, Ordering::SeqCst); self.0.notify(); }
                }
                let _busy = Busy(inner);
                inner.notify();
                let mut duration = None;
                let result = transcribe(&request, &mut stream, deadline, generation, inner, &mut duration);
                inner.record(request.header("origin").map(str::to_string), duration, &result);
                result.map(|transcript| Response::json(200, serde_json::to_value(transcript).unwrap_or_default()))
            }
            _ => Err(ApiError::new(404, "not_found", "Unknown local API route.")),
        }
    });
    let response = result.unwrap_or_else(|error| Response::json(error.status,
        serde_json::json!({"error":{"code":error.code,"message":error.message}})));
    respond(&mut stream, response, cors.as_deref());
}

fn transcribe(request: &Request, stream: &mut TcpStream, deadline: Instant, generation: u64,
    inner: &Inner, duration: &mut Option<u64>) -> Result<Transcript, ApiError>
{
    if request.header("content-type") != Some("audio/wav") {
        return Err(ApiError::new(415, "unsupported_media", "Use Content-Type: audio/wav."));
    }
    let length = request.header("content-length").filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(malformed)?.parse::<usize>().map_err(|_| ApiError::too_large())?;
    if length > audio::MAX_BYTES { return Err(ApiError::too_large()); }
    if length == 0 { return Err(ApiError::new(400, "bad_audio", "Upload a nonempty WAV file.")); }
    let language = request.header("x-lectus-language").map(str::to_string);
    if language.as_deref().is_some_and(|l| l != "auto" && (!(2..=3).contains(&l.len()) || !l.bytes().all(|b| b.is_ascii_lowercase()))) {
        return Err(ApiError::new(400, "bad_language", "Use auto or a supported lowercase language code."));
    }
    let cleanup = match request.header("x-lectus-cleanup") {
        None | Some("0") => false,
        Some("1") => true,
        _ => return Err(ApiError::new(400, "bad_cleanup", "X-Lectus-Cleanup must be 0 or 1.")),
    };
    let mut bytes = vec![0u8; length];
    let mut received = 0;
    while received < length { received += read_chunk(stream, &mut bytes[received..], deadline)?; }
    // Disabling, rotating the token, or changing the allowlist during upload
    // revokes this request before any decoder or inference work begins.
    if !inner.current(generation) { return Err(unauthorized()); }
    let (samples, audio_ms) = audio::decode(&bytes)?;
    *duration = Some(audio_ms);
    if !inner.current(generation) { return Err(unauthorized()); }
    if !inner.backend.allowed() { return Err(ApiError::locked()); }
    if inner.backend.busy() { return Err(ApiError::busy()); }
    if !inner.backend.ready() { return Err(ApiError::unavailable()); }
    if !inner.admit(generation) { return Err(unauthorized()); }
    struct Processing<'a>(&'a Inner);
    impl Drop for Processing<'_> {
        fn drop(&mut self) { self.0.processing.store(false, Ordering::SeqCst); self.0.notify(); }
    }
    let _processing = Processing(inner);
    inner.notify();
    // Backend errors are restricted to stable public messages. Do not expose
    // engine errors containing paths, prompts, or any transcript fragments.
    inner.backend.transcribe(samples, language, cleanup).map_err(|error| match error.code.as_str() {
        "busy" => ApiError::busy(),
        "locked" => ApiError::locked(),
        "bad_language" => ApiError::new(400, "bad_language", "The requested language is not supported."),
        _ => ApiError::unavailable(),
    })
}

struct Response { status: u16, body: Vec<u8>, preflight: bool, private_network: bool }
impl Response {
    fn json(status: u16, value: serde_json::Value) -> Self {
        Self { status, body: serde_json::to_vec(&value).unwrap_or_default(), preflight: false, private_network: false }
    }
}

fn preflight(request: &Request, origin: Option<&str>) -> Result<Response, ApiError> {
    let method = match request.path.as_str() { "/v1/transcribe" => "POST", "/v1/health" => "GET", _ => return Err(malformed()) };
    if origin.is_none() || request.header("access-control-request-method") != Some(method) { return Err(malformed()); }
    let headers: Vec<_> = request.header("access-control-request-headers").unwrap_or("").split(',')
        .map(|h| h.trim().to_ascii_lowercase()).collect();
    if !headers.iter().any(|h| h == "authorization") || headers.iter().any(|h|
        !matches!(h.as_str(), "authorization" | "content-type" | "x-lectus-language" | "x-lectus-cleanup"))
    { return Err(malformed()); }
    Ok(Response { status: 204, body: vec![], preflight: true,
        private_network: request.header("access-control-request-private-network") == Some("true") })
}

fn respond(stream: &mut TcpStream, response: Response, origin: Option<&str>) {
    let reason = match response.status { 200 => "OK", 204 => "No Content", _ => "Error" };
    let mut headers = format!("HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n", response.status, reason, response.body.len());
    if let Some(origin) = origin {
        headers.push_str(&format!("Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\n"));
        if response.preflight {
            headers.push_str("Access-Control-Allow-Methods: GET, POST\r\nAccess-Control-Allow-Headers: Authorization, Content-Type, X-Lectus-Language, X-Lectus-Cleanup\r\nAccess-Control-Max-Age: 0\r\n");
            if response.private_network { headers.push_str("Access-Control-Allow-Private-Network: true\r\n"); }
        }
    }
    headers.push_str("\r\n");
    // Small JSON responses are bounded separately from upload time; a stalled
    // reader cannot retain one of the four socket workers indefinitely.
    let deadline = Instant::now() + Duration::from_secs(5);
    let bytes = [headers.as_bytes(), &response.body].concat();
    let mut sent = 0;
    while sent < bytes.len() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() || stream.set_write_timeout(Some(remaining.min(Duration::from_secs(1)))).is_err() { return; }
        match stream.write(&bytes[sent..]) {
            Ok(0) => return,
            Ok(count) => sent += count,
            Err(error) if matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted) => {},
            Err(_) => return,
        }
    }
    finish_response(stream);
}

/// RFC 9112 section 9.6: immediately dropping a socket with an unread upload
/// can reset the connection and erase the response in the client's buffers.
/// Send FIN after the complete response, then briefly discard pending input.
/// This never decodes rejected audio or admits another request on the socket.
fn finish_response(stream: &mut TcpStream) {
    if stream.shutdown(Shutdown::Write).is_err() { return; }
    let deadline = Instant::now() + CLOSE_TIMEOUT;
    let mut buffer = [0u8; 8192];
    let mut discarded = 0usize;
    while discarded < audio::MAX_BYTES {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() || stream.set_read_timeout(Some(remaining.min(Duration::from_millis(50)))).is_err() { return; }
        let available = (audio::MAX_BYTES - discarded).min(buffer.len());
        match stream.read(&mut buffer[..available]) {
            Ok(0) => return,
            Ok(count) => discarded += count,
            Err(error) if matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted) => {},
            Err(_) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn response_teardown_is_bounded_when_the_peer_keeps_its_socket_open() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let _client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut socket, _) = listener.accept().unwrap();
        let start = Instant::now();
        finish_response(&mut socket);
        assert!(start.elapsed() >= CLOSE_TIMEOUT);
        assert!(start.elapsed() < Duration::from_secs(1), "teardown waited too long for a peer that never closed");
    }

    #[test]
    fn header_deadline_expires_even_when_a_client_keeps_dripping_bytes() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut socket, _) = listener.accept().unwrap();
        let writer = std::thread::spawn(move || {
            for _ in 0..12 {
                if client.write_all(b"G").is_err() { break; }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let start = Instant::now();
        let error = match read_request(&mut socket, start + Duration::from_millis(80)) {
            Ok(_) => panic!("incomplete header accepted"),
            Err(error) => error,
        };
        assert_eq!(error.status, 408);
        assert!(start.elapsed() < Duration::from_secs(1));
        drop(socket);
        writer.join().unwrap();
    }
}
