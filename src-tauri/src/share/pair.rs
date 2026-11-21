//! First-time pairing for Lectus Link with a 6-digit code shown on the receiving computer.
//!
//! SPAKE2 turns the short code into a 32-byte key. A listener on the network learns nothing
//! that lets it test codes offline, and an active guesser gets one try per connection; the
//! receiver replaces the code after three failures. Both sides then prove the key by sealing
//! their identity with it, and keep the key as their long-term pairing.
use crate::remote_dictation::transport::{open, random, read_frame, seal, write_frame, Pairing};
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use spake2::{Ed25519Group, Identity, Password, Spake2};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub const MAGIC: &[u8; 8] = b"LECPAIR1";
const ID_MAIN: &[u8] = b"lectus-link-main";
const ID_RECEIVER: &[u8] = b"lectus-link-receiver";
const MAX_SPAKE_MSG: usize = 64;

/// Who is on the other end, as each side stores it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hello {
    pub id: String,
    pub name: String,
}

pub fn new_code() -> Result<String> {
    let n = u32::from_le_bytes(random::<4>()?) % 1_000_000;
    Ok(format!("{n:06}"))
}

fn normalize(code: &str) -> String {
    code.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn write_msg(stream: &mut TcpStream, msg: &[u8]) -> Result<()> {
    stream.write_all(&(msg.len() as u16).to_be_bytes())?;
    stream.write_all(msg)?;
    Ok(())
}

fn read_msg(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut len = [0; 2];
    stream.read_exact(&mut len)?;
    let len = u16::from_be_bytes(len) as usize;
    if len == 0 || len > MAX_SPAKE_MSG {
        bail!("Invalid pairing message");
    }
    let mut msg = vec![0; len];
    stream.read_exact(&mut msg)?;
    Ok(msg)
}

fn key_from(raw: Vec<u8>) -> Result<[u8; 32]> {
    raw.try_into().map_err(|_| anyhow!("Unexpected pairing key size"))
}

fn prepare(stream: &TcpStream) -> Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    Ok(())
}

/// Main side. Returns the receiver's identity and the shared pairing (keyed, receiver's id).
pub fn pair_with(address: SocketAddr, code: &str, me: &Hello) -> Result<(Hello, Pairing)> {
    let code = normalize(code);
    if code.len() != 6 {
        bail!("Type the 6 digits shown on the other computer");
    }
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(900))?;
    prepare(&stream)?;
    let (spake, outbound) = Spake2::<Ed25519Group>::start_a(
        &Password::new(code.as_bytes()),
        &Identity::new(ID_MAIN),
        &Identity::new(ID_RECEIVER),
    );
    stream.write_all(MAGIC)?;
    write_msg(&mut stream, &outbound)?;
    let inbound = read_msg(&mut stream)?;
    let key = key_from(spake.finish(&inbound).map_err(|_| anyhow!("Pairing failed"))?)?;
    let temp = Pairing::from_key(String::new(), key);
    write_frame(&mut stream, &seal(&temp, b"pair-main", me)?)?;
    let wrong = || anyhow!("Wrong code. Check the 6 digits on the other computer and try again");
    let reply = read_frame(&mut stream).map_err(|_| wrong())?;
    let them: Hello = open(&temp, b"pair-receiver", &reply).map_err(|_| wrong())?;
    let pairing = Pairing::from_key(them.id.clone(), key);
    Pairing::parse(&pairing.code()).map_err(|_| anyhow!("The other computer sent an invalid id"))?;
    Ok((them, pairing))
}

/// Receiver side, after the magic was read. `Ok(None)` means the code was wrong.
pub fn serve(stream: &mut TcpStream, code: &str, me: &Hello) -> Result<Option<(Hello, [u8; 32])>> {
    prepare(stream)?;
    let inbound = read_msg(stream)?;
    let (spake, outbound) = Spake2::<Ed25519Group>::start_b(
        &Password::new(normalize(code).as_bytes()),
        &Identity::new(ID_MAIN),
        &Identity::new(ID_RECEIVER),
    );
    write_msg(stream, &outbound)?;
    let Ok(raw) = spake.finish(&inbound) else {
        return Ok(None);
    };
    let key = key_from(raw)?;
    let temp = Pairing::from_key(String::new(), key);
    let Ok(them) = open::<Hello>(&temp, b"pair-main", &read_frame(stream)?) else {
        return Ok(None); // different code on the other side
    };
    write_frame(stream, &seal(&temp, b"pair-receiver", me)?)?;
    Ok(Some((them, key)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn hello(id: &str, name: &str) -> Hello {
        Hello { id: id.into(), name: name.into() }
    }

    fn run(receiver_code: &'static str, typed: &str) -> (Result<(Hello, Pairing)>, Option<(Hello, [u8; 32])>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let receiver_id = Pairing::generate().unwrap().id;
        let rid = receiver_id.clone();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut magic = [0; 8];
            stream.read_exact(&mut magic).unwrap();
            assert_eq!(&magic, MAGIC);
            serve(&mut stream, receiver_code, &hello(&rid, "Mac")).unwrap()
        });
        let main = pair_with(address, typed, &hello("main-id", "PC"));
        (main, server.join().unwrap())
    }

    #[test]
    fn same_code_gives_both_sides_the_same_key() {
        let (main, receiver) = run("123456", "123 456");
        let (them, pairing) = main.unwrap();
        let (main_hello, key) = receiver.unwrap();
        assert_eq!(them.name, "Mac");
        assert_eq!(main_hello, hello("main-id", "PC"));
        assert_eq!(pairing.code(), Pairing::from_key(them.id, key).code());
    }

    #[test]
    fn wrong_code_pairs_nobody() {
        let (main, receiver) = run("123456", "654321");
        assert!(main.is_err());
        assert!(receiver.is_none());
    }

    #[test]
    fn short_code_is_refused_before_connecting() {
        let err = pair_with("127.0.0.1:9".parse().unwrap(), "12345", &hello("a", "b")).err().unwrap();
        assert!(err.to_string().contains("6 digits"));
    }

    #[test]
    fn codes_have_six_digits() {
        for _ in 0..50 {
            let c = new_code().unwrap();
            assert_eq!(c.len(), 6);
            assert!(c.chars().all(|ch| ch.is_ascii_digit()));
        }
    }
}
