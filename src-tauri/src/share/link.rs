//! Encrypted, ordered message link for Lectus Share, keyed by the remote dictation pairing.
//!
//! Handshake: the PC sends a magic and a fresh nonce; the Mac answers with both nonces sealed
//! under the pairing key, which proves it holds the key. Every later frame carries the nonce
//! chosen by its receiver and a strictly increasing counter, so frames cannot be replayed
//! across connections or reordered within one. Each direction has its own label.
use crate::remote_dictation::transport::{open, random, read_frame, seal, write_frame, Pairing};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

pub const MAGIC: &[u8; 8] = b"LECLNK02";
/// Both sides send a Ping every second; three silent seconds mean the link is gone.
pub const PING_EVERY: Duration = Duration::from_secs(1);
pub const SILENCE_LIMIT: Duration = Duration::from_secs(3);

/// A screen edge. In settings it says where the other computer is; in messages it is the
/// edge of the receiving screen that faces the main computer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
}

impl Edge {
    pub fn opposite(self) -> Edge {
        match self {
            Edge::Left => Edge::Right,
            Edge::Right => Edge::Left,
            Edge::Top => Edge::Bottom,
            Edge::Bottom => Edge::Top,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Msg {
    /// Main to receiver: the cursor enters through edge `e` of the receiving screen, at `at`
    /// along it (0 = top or left end, 1 = bottom or right end).
    Enter { e: Edge, at: f64 },
    /// Main to receiver: relative motion in main-screen pixels.
    Move { dx: i32, dy: i32 },
    /// Main to receiver: 0 left, 1 right, 2 middle.
    Button { b: u8, down: bool },
    /// Main to receiver: Windows wheel units, 120 per notch. Positive `dy` scrolls up, `dx` right.
    Wheel { dx: i32, dy: i32 },
    /// Main to receiver: a key by position: set-1 scan code `s`, E0-extended `e`.
    Key { s: u16, e: bool, down: bool },
    /// Main to receiver: insert this dictation once. Answered by `Ack` with the same id.
    Text { id: [u8; 16], text: String },
    /// Main to receiver: the main computer took the mouse back (escape key or shutdown).
    Leave,
    /// Receiver to main: the cursor crossed back at `at` along the shared edge.
    Left { at: f64 },
    /// Receiver to main: result of a `Text`. `error` is None when the text was inserted.
    Ack { id: [u8; 16], error: Option<String> },
    Ping,
}

#[derive(Serialize, Deserialize)]
struct Hello {
    pc: [u8; 32],
    mac: [u8; 32],
}

#[derive(Serialize, Deserialize)]
struct Frame {
    s: [u8; 32],
    n: u64,
    m: Msg,
}

/// Which end of the link this is. The PC controls, the Mac is controlled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Pc,
    Mac,
}

pub struct Session {
    pub stream: TcpStream,
    pairing: Pairing,
    side: Side,
    /// Nonce our peer chose: stamped on what we send.
    peer_nonce: [u8; 32],
    /// Nonce we chose: required on what we receive.
    own_nonce: [u8; 32],
    sent: u64,
    received: u64,
}

fn labels(side: Side) -> (&'static [u8], &'static [u8]) {
    match side {
        Side::Pc => (b"share-pc", b"share-mac"),
        Side::Mac => (b"share-mac", b"share-pc"),
    }
}

fn prepare(stream: &TcpStream) -> Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(SILENCE_LIMIT))?;
    stream.set_write_timeout(Some(SILENCE_LIMIT))?;
    Ok(())
}

impl Session {
    /// Splits off a second handle for a reader thread. Counters stay per handle and per
    /// direction, so one handle must only send and the other only receive.
    pub fn split(&self) -> Result<Session> {
        Ok(Session {
            stream: self.stream.try_clone()?,
            pairing: self.pairing.clone(),
            side: self.side,
            peer_nonce: self.peer_nonce,
            own_nonce: self.own_nonce,
            sent: self.sent,
            received: self.received,
        })
    }

    pub fn send(&mut self, msg: Msg) -> Result<()> {
        self.sent += 1;
        let frame = seal(
            &self.pairing,
            labels(self.side).0,
            &Frame {
                s: self.peer_nonce,
                n: self.sent,
                m: msg,
            },
        )?;
        write_frame(&mut self.stream, &frame)
    }

    pub fn recv(&mut self) -> Result<Msg> {
        let data = read_frame(&mut self.stream)?;
        let frame: Frame = open(&self.pairing, labels(self.side).1, &data)?;
        if frame.s != self.own_nonce || frame.n != self.received + 1 {
            bail!("out-of-order or replayed frame");
        }
        self.received = frame.n;
        Ok(frame.m)
    }

    pub fn close(&self) {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }
}

pub fn connect(address: SocketAddr, pairing: &Pairing) -> Result<Session> {
    let stream = TcpStream::connect_timeout(&address, Duration::from_millis(900))?;
    handshake_pc(stream, pairing)
}

fn handshake_pc(mut stream: TcpStream, pairing: &Pairing) -> Result<Session> {
    prepare(&stream)?;
    let pc = random::<32>()?;
    stream.write_all(MAGIC)?;
    stream.write_all(&pc)?;
    let hello: Hello = open(pairing, b"share-hello", &read_frame(&mut stream)?)?;
    if hello.pc != pc {
        bail!("The Mac did not prove the pairing");
    }
    Ok(Session {
        stream,
        pairing: pairing.clone(),
        side: Side::Pc,
        peer_nonce: hello.mac,
        own_nonce: pc,
        sent: 0,
        received: 0,
    })
}

pub fn accept(mut stream: TcpStream, pairing: &Pairing) -> Result<Session> {
    // Pairing traffic is routed elsewhere before this point; see `super::pair`.
    prepare(&stream)?;
    let mut magic = [0; 8];
    stream.read_exact(&mut magic)?;
    if &magic != MAGIC {
        bail!("Not a Lectus Share connection");
    }
    let mut pc = [0; 32];
    stream.read_exact(&mut pc)?;
    let mac = random::<32>()?;
    write_frame(&mut stream, &seal(pairing, b"share-hello", &Hello { pc, mac })?)?;
    Ok(Session {
        stream,
        pairing: pairing.clone(),
        side: Side::Mac,
        peer_nonce: pc,
        own_nonce: mac,
        sent: 0,
        received: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn pair_with(pairing: &Pairing) -> (Session, Session) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let p = pairing.clone();
        let mac = std::thread::spawn(move || accept(listener.accept().unwrap().0, &p).unwrap());
        let pc = connect(address, pairing).unwrap();
        (pc, mac.join().unwrap())
    }

    fn pair() -> (Session, Session) {
        pair_with(&Pairing::generate().unwrap())
    }

    #[test]
    fn messages_flow_both_ways_in_order() {
        let (mut pc, mut mac) = pair();
        pc.send(Msg::Enter { e: Edge::Right, at: 0.5 }).unwrap();
        pc.send(Msg::Move { dx: -3, dy: 7 }).unwrap();
        pc.send(Msg::Button { b: 0, down: true }).unwrap();
        assert_eq!(mac.recv().unwrap(), Msg::Enter { e: Edge::Right, at: 0.5 });
        assert_eq!(mac.recv().unwrap(), Msg::Move { dx: -3, dy: 7 });
        assert_eq!(mac.recv().unwrap(), Msg::Button { b: 0, down: true });
        mac.send(Msg::Left { at: 0.25 }).unwrap();
        assert_eq!(pc.recv().unwrap(), Msg::Left { at: 0.25 });
        let text = Msg::Text { id: [7; 16], text: "Olá, mundo".into() };
        pc.send(text.clone()).unwrap();
        assert_eq!(mac.recv().unwrap(), text);
    }

    #[test]
    fn split_handles_send_and_receive_independently() {
        let (pc, mut mac) = pair();
        let mut reader = pc.split().unwrap();
        let mut writer = pc;
        writer.send(Msg::Ping).unwrap();
        assert_eq!(mac.recv().unwrap(), Msg::Ping);
        mac.send(Msg::Ping).unwrap();
        assert_eq!(reader.recv().unwrap(), Msg::Ping);
    }

    #[test]
    fn wrong_pairing_is_rejected_at_handshake() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let mac_pairing = Pairing::generate().unwrap();
        let mac = std::thread::spawn(move || {
            let _ = accept(listener.accept().unwrap().0, &mac_pairing);
        });
        assert!(connect(address, &Pairing::generate().unwrap()).is_err());
        mac.join().unwrap();
    }

    #[test]
    fn a_frame_from_another_session_is_rejected() {
        let pairing = Pairing::generate().unwrap();
        let (pc1, _mac1) = pair_with(&pairing);
        let (mut pc2, mut mac2) = pair_with(&pairing);
        // A valid frame recorded on session 1, replayed raw into session 2.
        let frame = seal(
            &pairing,
            b"share-pc",
            &Frame { s: pc1.peer_nonce, n: 1, m: Msg::Leave },
        )
        .unwrap();
        write_frame(&mut pc2.stream, &frame).unwrap();
        assert!(mac2.recv().is_err());
    }

    #[test]
    fn replayed_counter_is_rejected() {
        let (pc, mut mac) = pair();
        let mut a = pc.split().unwrap();
        let mut b = pc;
        a.send(Msg::Ping).unwrap(); // n = 1
        b.send(Msg::Ping).unwrap(); // n = 1 again from a second handle: a replay
        assert_eq!(mac.recv().unwrap(), Msg::Ping);
        assert!(mac.recv().is_err());
    }
}
