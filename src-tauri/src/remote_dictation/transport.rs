//! Bounded, mutually authenticated text-only transport. No clipboard or Tauri dependencies.
use anyhow::{anyhow, bail, Context, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

const MAGIC: &[u8; 8] = b"LECTXT01";
pub const MAX_TEXT: usize = 64 * 1024;
const MAX_FRAME: usize = MAX_TEXT * 6 + 2048; // JSON may escape each control character.
const IO_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Clone)]
pub struct Pairing {
    pub id: String,
    key: [u8; 32],
}

fn random<const N: usize>() -> Result<[u8; N]> {
    let mut bytes = [0; N];
    getrandom::getrandom(&mut bytes).map_err(|_| anyhow!("Random generator unavailable"))?;
    Ok(bytes)
}

impl Pairing {
    pub fn generate() -> Result<Self> {
        Ok(Self { id: bs58::encode(random::<12>()?).into_string(), key: random()? })
    }

    pub fn code(&self) -> String {
        format!("LECTUS1.{}.{}", self.id, bs58::encode(self.key).into_string())
    }

    pub fn parse(code: &str) -> Result<Self> {
        let parts: Vec<_> = code.trim().split('.').collect();
        if parts.len() != 3 || parts[0] != "LECTUS1" {
            bail!("Paste the complete connection code from the receiving computer");
        }
        let id = bs58::decode(parts[1]).into_vec().map_err(|_| anyhow!("Invalid connection code"))?;
        let key = bs58::decode(parts[2]).into_vec().map_err(|_| anyhow!("Invalid connection code"))?;
        if id.len() != 12 || key.len() != 32 {
            bail!("Invalid connection code");
        }
        Ok(Self {
            id: parts[1].to_owned(),
            key: key.try_into().map_err(|_| anyhow!("Invalid connection code"))?,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct Hello {
    client: [u8; 32],
    server: [u8; 32],
}

#[derive(Serialize, Deserialize)]
struct Request {
    challenge: [u8; 32],
    id: [u8; 16],
    text: Option<String>, // None is a connection check: never injects text.
}

#[derive(Serialize, Deserialize)]
struct Reply {
    id: [u8; 16],
    error: Option<String>,
}

fn seal<T: Serialize>(pairing: &Pairing, kind: &[u8], value: &T) -> Result<Vec<u8>> {
    let nonce = random::<24>()?;
    let plain = serde_json::to_vec(value)?;
    let cipher = XChaCha20Poly1305::new((&pairing.key).into());
    let encrypted = cipher.encrypt(XNonce::from_slice(&nonce), Payload { msg: &plain, aad: kind })
        .map_err(|_| anyhow!("Unable to encrypt delivery"))?;
    let mut frame = nonce.to_vec();
    frame.extend(encrypted);
    if frame.len() > MAX_FRAME { bail!("Dictation is too long to send"); }
    Ok(frame)
}

fn open<T: DeserializeOwned>(pairing: &Pairing, kind: &[u8], frame: &[u8]) -> Result<T> {
    if frame.len() < 40 || frame.len() > MAX_FRAME { bail!("Invalid message size"); }
    let cipher = XChaCha20Poly1305::new((&pairing.key).into());
    let plain = cipher.decrypt(XNonce::from_slice(&frame[..24]),
        Payload { msg: &frame[24..], aad: kind })
        .map_err(|_| anyhow!("The connection code does not match"))?;
    serde_json::from_slice(&plain).context("Invalid delivery message")
}

fn write_frame(stream: &mut impl Write, frame: &[u8]) -> Result<()> {
    if frame.len() > MAX_FRAME { bail!("Message too large"); }
    stream.write_all(&(frame.len() as u32).to_be_bytes())?;
    stream.write_all(frame)?;
    Ok(())
}

fn read_frame(stream: &mut impl Read) -> Result<Vec<u8>> {
    let mut size = [0; 4];
    stream.read_exact(&mut size)?;
    let size = u32::from_be_bytes(size) as usize;
    if !(40..=MAX_FRAME).contains(&size) { bail!("Invalid message size"); }
    let mut data = vec![0; size];
    stream.read_exact(&mut data)?;
    Ok(data)
}

fn prepare(stream: &TcpStream) -> Result<()> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    stream.set_nodelay(true)?;
    Ok(())
}

fn validate_request(request: &Request, challenge: &[u8; 32]) -> Result<()> {
    if request.challenge != *challenge { bail!("Expired delivery challenge"); }
    if let Some(text) = &request.text {
        if text.trim().is_empty() || text.len() > MAX_TEXT {
            bail!("Dictation is empty or too long");
        }
    }
    Ok(())
}

/// One connection, one request. Fresh server challenge prevents replay across connections.
/// The callback must not queue a later paste: returning success means the paste completed.
pub fn receive(
    mut stream: TcpStream,
    pairing: &Pairing,
    deliver: impl FnOnce(Option<&str>) -> Result<()>,
) -> Result<()> {
    prepare(&stream)?;
    let mut magic = [0; 8];
    stream.read_exact(&mut magic)?;
    if &magic != MAGIC { bail!("Not a Lectus connection"); }
    let mut client = [0; 32];
    stream.read_exact(&mut client)?;
    let server = random()?;
    write_frame(&mut stream, &seal(pairing, b"hello", &Hello { client, server })?)?;
    let request: Request = open(pairing, b"request", &read_frame(&mut stream)?)?;
    validate_request(&request, &server)?;
    // Authenticated errors go to the paired sender only. Never log text or the secret.
    let error = deliver(request.text.as_deref()).err().map(|e| e.to_string());
    write_frame(&mut stream, &seal(pairing, b"reply", &Reply { id: request.id, error })?)?;
    Ok(())
}

fn connect(address: SocketAddr, pairing: &Pairing) -> Result<(TcpStream, [u8; 32])> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(900))?;
    prepare(&stream)?;
    let client = random()?;
    stream.write_all(MAGIC)?;
    stream.write_all(&client)?;
    let hello: Hello = open(pairing, b"hello", &read_frame(&mut stream)?)?;
    if hello.client != client { bail!("Receiver authentication failed"); }
    Ok((stream, hello.server))
}

/// Tries alternate discovered addresses only BEFORE submitting text.
/// A missing acknowledgement after submission must never cause a duplicate paste.
pub fn send(addresses: &[SocketAddr], pairing: &Pairing, text: Option<&str>) -> Result<()> {
    if let Some(text) = text {
        if text.trim().is_empty() || text.len() > MAX_TEXT { bail!("Dictation is empty or too long"); }
    }
    let mut connected = None;
    for address in addresses.iter().take(6) {
        if let Ok(connection) = connect(*address, pairing) {
            connected = Some(connection);
            break;
        }
    }
    let (mut stream, challenge) = connected
        .ok_or_else(|| anyhow!("Cannot reach the paired computer. Open Lectus there in Receive mode and check the connection code and local-network permission."))?;
    let request = Request { challenge, id: random()?, text: text.map(str::to_owned) };
    let frame = seal(pairing, b"request", &request)?;
    // From this point onward the receiver may have inserted text. Never auto-retry.
    let result = (|| -> Result<()> {
        write_frame(&mut stream, &frame)?;
        let reply: Reply = open(pairing, b"reply", &read_frame(&mut stream)?)?;
        if reply.id != request.id { bail!("Unexpected delivery acknowledgement"); }
        if let Some(error) = reply.error { return Err(anyhow!("Receiving computer: {error}")); }
        Ok(())
    })();
    result.map_err(|e| anyhow!(
        "{e}. Delivery was not confirmed; check the receiving app before trying again. The dictation is saved in History."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{net::TcpListener, sync::{Arc, Mutex}, thread};

    fn server(pair: Pairing, callback: impl FnOnce(Option<&str>) -> Result<()> + Send + 'static)
        -> (SocketAddr, thread::JoinHandle<Result<()>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || receive(listener.accept().unwrap().0, &pair, callback));
        (address, handle)
    }

    #[test]
    fn pairing_round_trip_and_rejects_invalid_codes() {
        let p = Pairing::generate().unwrap();
        assert_eq!(Pairing::parse(&p.code()).unwrap().key, p.key);
        for code in ["", "LECTUS1.a.b", "LECTUS2.abc.def"] {
            assert!(Pairing::parse(code).is_err());
        }
    }

    #[test]
    fn ciphertext_rejects_wrong_key_tampering_and_message_reflection() {
        let p = Pairing::generate().unwrap();
        let frame = seal(&p, b"request", &"private text").unwrap();
        assert!(!frame.windows(12).any(|w| w == b"private text"));
        assert!(open::<String>(&Pairing::generate().unwrap(), b"request", &frame).is_err());
        assert!(open::<String>(&p, b"reply", &frame).is_err());
        let mut corrupt = frame;
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(open::<String>(&p, b"request", &corrupt).is_err());
    }

    #[test]
    fn replay_is_rejected_with_a_new_connection_challenge() {
        let request = Request { challenge: [1; 32], id: [2; 16], text: Some("hello".into()) };
        assert!(validate_request(&request, &[1; 32]).is_ok());
        assert!(validate_request(&request, &[3; 32]).is_err());
    }

    #[test]
    fn rejects_oversized_frame_before_allocating() {
        let mut input = std::io::Cursor::new(u32::MAX.to_be_bytes());
        assert!(read_frame(&mut input).is_err());
        assert!(send(&[], &Pairing::generate().unwrap(), Some(&"x".repeat(MAX_TEXT + 1))).is_err());
    }

    #[test]
    fn paired_delivery_inserts_once_and_acknowledges_unicode() {
        let p = Pairing::generate().unwrap();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let dest = captured.clone();
        let (addr, task) = server(p.clone(), move |text| {
            dest.lock().unwrap().push(text.unwrap().to_owned()); Ok(())
        });
        send(&[addr], &p, Some("Olá João — ditado no Mac")).unwrap();
        task.join().unwrap().unwrap();
        assert_eq!(*captured.lock().unwrap(), vec!["Olá João — ditado no Mac"]);
    }

    #[test]
    fn connection_check_does_not_insert_text() {
        let p = Pairing::generate().unwrap();
        let (addr, task) = server(p.clone(), |text| { assert!(text.is_none()); Ok(()) });
        send(&[addr], &p, None).unwrap();
        task.join().unwrap().unwrap();
    }

    #[test]
    fn wrong_pairing_never_reaches_delivery_callback() {
        let p = Pairing::generate().unwrap();
        let (addr, task) = server(p, |_| panic!("unauthenticated insertion"));
        assert!(send(&[addr], &Pairing::generate().unwrap(), Some("secret")).is_err());
        assert!(task.join().unwrap().is_err());
    }

    #[test]
    fn receiver_error_is_propagated_without_fallback() {
        let p = Pairing::generate().unwrap();
        let (addr, task) = server(p.clone(), |_| bail!("Accessibility permission is off"));
        let error = send(&[addr], &p, Some("hello")).unwrap_err().to_string();
        assert!(error.contains("Accessibility"));
        task.join().unwrap().unwrap();
    }

    #[test]
    fn lost_acknowledgement_never_retries_another_address() {
        let p = Pairing::generate().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let receiver_pair = p.clone();
        let task = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            prepare(&stream).unwrap();
            let mut header = [0; 40]; stream.read_exact(&mut header).unwrap();
            let client = header[8..].try_into().unwrap();
            let server = random().unwrap();
            write_frame(&mut stream, &seal(&receiver_pair, b"hello", &Hello {client, server}).unwrap()).unwrap();
            let request: Request = open(&receiver_pair, b"request", &read_frame(&mut stream).unwrap()).unwrap();
            validate_request(&request, &server).unwrap();
            // Simulate a successful insertion followed by a lost response.
            drop(stream);
            listener.set_nonblocking(true).unwrap();
            thread::sleep(Duration::from_millis(100));
            assert!(listener.accept().is_err(), "sender must not reconnect after submission");
        });
        let error = send(&[addr, addr], &p, Some("once")).unwrap_err().to_string();
        assert!(error.contains("not confirmed"));
        task.join().unwrap();
    }
}

