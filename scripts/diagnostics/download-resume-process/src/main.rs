//! Isolated process-restart diagnostic. The transport is the production file;
//! app-state events, Tauri commands and native app shutdown are outside scope.
#[path = "../../../../src-tauri/src/model_download.rs"]
mod transport;

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::{io::{Read, Write}, net::TcpListener, path::Path, sync::atomic::AtomicBool, time::Duration};

fn serve(directory: &Path) -> Result<()> {
    let bytes: Vec<u8> = (0..2_000_000).map(|n| (n % 251) as u8).collect();
    let checksum = format!("{:x}", Sha256::digest(&bytes));
    let listener = TcpListener::bind("127.0.0.1:0")?;
    std::fs::write(directory.join("server.json"), serde_json::to_vec(&serde_json::json!({
        "url": format!("http://{}/fixture", listener.local_addr()?), "sha256": checksum, "size": bytes.len()
    }))?)?;
    let mut requests = std::fs::OpenOptions::new().create_new(true).write(true).open(directory.join("requests.jsonl"))?;
    // The launcher kills one downloader and starts another. Two completed
    // header requests are sufficient; unexpected extra connections fail.
    for attempt in 0..2 {
        let (mut stream, _) = listener.accept()?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            if stream.read(&mut byte)? == 0 { return Err(anyhow!("Client closed before headers")); }
            headers.push(byte[0]);
            if headers.len() > 8192 { return Err(anyhow!("Unexpected oversized fixture request")); }
        }
        let headers = String::from_utf8(headers)?.to_lowercase();
        let range = headers.lines().find_map(|line| line.strip_prefix("range: bytes="));
        let offset = range.map(|range| range.trim_end_matches('-').parse::<usize>()).transpose()?.unwrap_or(0);
        let if_range = headers.lines().find_map(|line| line.strip_prefix("if-range: "));
        if attempt == 0 && offset != 0 { return Err(anyhow!("First request unexpectedly resumed")); }
        if attempt == 1 && (offset == 0 || if_range != Some("\"process-fixture-v1\"")) {
            return Err(anyhow!("Restarted process did not send Range and matching If-Range"));
        }
        if offset >= bytes.len() { return Err(anyhow!("Unexpected fixture range")); }
        writeln!(requests, "{}", serde_json::json!({"attempt": attempt, "offset": offset, "if_range": if_range}))?;
        requests.flush()?;
        let response = if offset > 0 {
            format!("HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {offset}-{}/{}\r\n", bytes.len()-1, bytes.len())
        } else { "HTTP/1.1 200 OK\r\n".to_owned() };
        write!(stream, "{response}Content-Length: {}\r\nETag: \"process-fixture-v1\"\r\nConnection: close\r\n\r\n", bytes.len()-offset)?;
        for chunk in bytes[offset..].chunks(8192) {
            if stream.write_all(chunk).is_err() { break; }
            // Gives the launcher a bounded interval to kill only this child.
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    Ok(())
}

fn download(url: &str, dest: &Path, expected: &str) -> Result<()> {
    let mut resumed_from = None;
    transport::transfer(url, dest, expected, &AtomicBool::new(false), |bytes, _, _| {
        resumed_from.get_or_insert(bytes);
    })?;
    let partial = dest.with_extension("part");
    let bytes = std::fs::read(&partial)?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != expected { return Err(anyhow!("Final digest mismatch; destination remains unpublished")); }
    // Harness publication uses the same digest-before-rename invariant. This
    // does not exercise Tauri's native publication/events or installed app.
    std::fs::rename(&partial, dest)?;
    transport::remove_receipt(dest);
    println!("{}", serde_json::json!({"completed": true, "resumed_from": resumed_from, "size": bytes.len(), "sha256": actual}));
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("serve") if args.len() == 3 => serve(Path::new(&args[2])),
        Some("download") if args.len() == 5 => download(&args[2], Path::new(&args[3]), &args[4]),
        _ => Err(anyhow!("Expected serve DIRECTORY or download URL DESTINATION SHA256")),
    }
}
