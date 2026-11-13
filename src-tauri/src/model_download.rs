//! Resumable transport. Only a fully sized, digest-verified partial may be published.
use anyhow::{anyhow, Context, Result};
use reqwest::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_RANGE, ETAG, IF_RANGE, LAST_MODIFIED, RANGE};
use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path, sync::atomic::{AtomicBool, Ordering}, time::Duration};

#[derive(Debug)]
pub(super) struct Paused;
impl std::fmt::Display for Paused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Download paused. Saved progress will resume when you continue.")
    }
}
impl std::error::Error for Paused {}

#[derive(Serialize, Deserialize)]
struct Resume {
    url: String,
    checksum: String,
    total: u64,
    validator: Option<String>,
}

pub(super) fn partial_status(dest: &Path) -> (u64, Option<u64>) {
    let bytes = std::fs::metadata(dest.with_extension("part")).map(|m| m.len()).unwrap_or(0);
    let saved = std::fs::read(dest.with_extension("part.json")).ok()
        .and_then(|b| serde_json::from_slice::<Resume>(&b).ok());
    (bytes, saved.map(|s| s.total))
}

fn validator(headers: &reqwest::header::HeaderMap) -> Option<String> {
    headers.get(ETAG).and_then(|v| v.to_str().ok()).filter(|v| !v.starts_with("W/"))
        .or_else(|| headers.get(LAST_MODIFIED).and_then(|v| v.to_str().ok()))
        .map(str::to_owned)
}

fn content_range(value: &str) -> Result<(u64, u64, u64)> {
    let (range, total) = value.strip_prefix("bytes ").and_then(|v| v.split_once('/'))
        .ok_or_else(|| anyhow!("The server returned an invalid download range. Progress was kept; retry."))?;
    let (start, end) = range.split_once('-').ok_or_else(|| anyhow!("Invalid download range"))?;
    let (start, end, total) = (start.parse::<u64>()?, end.parse::<u64>()?, total.parse::<u64>()?);
    if start > end || end >= total { return Err(anyhow!("Invalid download range")); }
    Ok((start, end, total))
}

fn check_space(available: u64, remaining: u64) -> Result<()> {
    if available < remaining.saturating_add(1024 * 1024) {
        return Err(anyhow!("Not enough free disk space: {remaining} more bytes are needed. Free space and resume; downloaded progress was kept."));
    }
    Ok(())
}

fn available_space(path: &Path) -> Result<u64> {
    #[cfg(windows)] {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        extern "system" { fn GetDiskFreeSpaceExW(path: *const u16, available: *mut u64, total: *mut u64, free: *mut u64) -> i32; }
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut available = 0;
        if unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, std::ptr::null_mut(), std::ptr::null_mut()) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(available)
    }
    #[cfg(unix)] {
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::CString::new(path.as_os_str().as_bytes())?;
        let mut info = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        if unsafe { libc::statvfs(path.as_ptr(), info.as_mut_ptr()) } != 0 { return Err(std::io::Error::last_os_error().into()); }
        let info = unsafe { info.assume_init() };
        Ok((info.f_bavail as u64).saturating_mul(info.f_frsize as u64))
    }
}

async fn cancellable<F: std::future::Future>(cancel: &AtomicBool, future: F) -> Result<F::Output> {
    tokio::pin!(future);
    loop {
        if cancel.load(Ordering::Acquire) { return Err(Paused.into()); }
        tokio::select! {
            value = &mut future => return Ok(value),
            _ = tokio::time::sleep(Duration::from_millis(100)) => {},
        }
    }
}

pub(super) fn transfer(
    url: &str, dest: &Path, checksum: &str, cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64, &str),
) -> Result<()> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let partial = dest.with_extension("part");
        let receipt = dest.with_extension("part.json");
        let saved = std::fs::read(&receipt).ok().and_then(|b| serde_json::from_slice::<Resume>(&b).ok());
        let bytes = std::fs::metadata(&partial).map(|m| m.len()).unwrap_or(0);
        let resume = saved.filter(|s| s.url == url && s.checksum == checksum && s.total >= bytes && s.validator.is_some());
        let offset = if resume.is_some() { bytes } else { 0 };
        // A fully downloaded partial may only need verification after a crash.
        if resume.as_ref().is_some_and(|s| offset > 100_000 && offset == s.total) {
            progress(offset, offset, "Verifying the saved download…");
            return Ok(());
        }
        let client = reqwest::Client::builder().connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30)).build()?;
        let mut request = client.get(url).header(ACCEPT_ENCODING, "identity");
        if offset > 0 {
            request = request.header(RANGE, format!("bytes={offset}-"))
                .header(IF_RANGE, resume.as_ref().unwrap().validator.as_deref().unwrap());
        }
        let mut response = cancellable(cancel, request.send()).await?
            .map_err(|_| anyhow!("Could not reach the model server. Check your connection and resume; saved progress was kept."))?;
        let status = response.status();
        if !status.is_success() { return Err(anyhow!("Model server returned HTTP {}. Saved progress was kept; retry later.", status.as_u16())); }
        if response.headers().get(CONTENT_ENCODING).is_some_and(|v| v != "identity") {
            return Err(anyhow!("The model server returned compressed bytes that cannot be resumed safely."));
        }
        let response_validator = validator(response.headers());
        let (start, total) = if status == reqwest::StatusCode::PARTIAL_CONTENT {
            let range = response.headers().get(CONTENT_RANGE).and_then(|v| v.to_str().ok())
                .ok_or_else(|| anyhow!("The model server did not confirm the resumed range. Saved progress was kept."))?;
            let (start, end, total) = content_range(range)?;
            if start != offset || end != total - 1 || response.content_length() != Some(total - start)
                || resume.as_ref().is_none_or(|s| s.total != total || s.validator != response_validator) {
                // Preserve bytes, but stop trusting this remote identity. The
                // next retry requests a fresh representation instead of looping
                // forever on the same inconsistent range.
                let _ = std::fs::remove_file(&receipt);
                return Err(anyhow!("The remote model changed or the server returned an inconsistent range. Retry to restart safely."));
            }
            (start, total)
        } else if status == reqwest::StatusCode::OK {
            (0, response.content_length().ok_or_else(|| anyhow!("The server did not provide the model size; download was refused safely."))?)
        } else { return Err(anyhow!("Unexpected model server response")); };
        if total <= 100_000 { return Err(anyhow!("The server response is too small to be a model.")); }
        check_space(available_space(dest.parent().unwrap_or(Path::new(".")))?, total - start)?;
        let message = if start > 0 { "Resuming saved download…" }
            else if bytes > 0 { "The server could not resume this file. Restarting safely…" }
            else { "Downloading model…" };
        progress(start, total, message);
        // Drop an incompatible receipt before truncating. A crash between these
        // writes can never attach old remote identity to new bytes.
        if start == 0 && receipt.exists() { std::fs::remove_file(&receipt)?; }
        let mut file = std::fs::OpenOptions::new().create(true).write(true)
            .truncate(start == 0).append(start > 0).open(&partial)?;
        if start == 0 {
            let metadata = Resume { url: url.to_owned(), checksum: checksum.to_owned(), total, validator: response_validator };
            let mut saved = std::fs::File::create(&receipt)?;
            saved.write_all(&serde_json::to_vec(&metadata)?)?;
            saved.sync_all()?;
        }
        let mut downloaded = start;
        loop {
            let chunk = cancellable(cancel, response.chunk()).await?
                .map_err(|_| anyhow!("Download interrupted. Check your connection and resume; saved progress was kept."))?;
            let Some(chunk) = chunk else { break };
            if downloaded.saturating_add(chunk.len() as u64) > total { return Err(anyhow!("The model server sent too much data. The file was not made available.")); }
            file.write_all(&chunk).context("Could not save the model. Check free space and permissions, then resume; saved progress was kept")?;
            downloaded += chunk.len() as u64;
            progress(downloaded, total, "Downloading model…");
        }
        if downloaded != total { return Err(anyhow!("Incomplete model download. Saved progress was kept; resume to finish.")); }
        file.sync_all().context("Could not finish saving the model. Check disk space and resume")?;
        if cancel.load(Ordering::Acquire) { return Err(Paused.into()); }
        progress(downloaded, total, "Verifying download…");
        Ok(())
    })
}

pub(super) fn remove_receipt(dest: &Path) { let _ = std::fs::remove_file(dest.with_extension("part.json")); }

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Read, net::TcpListener, sync::{Arc, Mutex}};
    fn server(replies: Vec<(String, Vec<u8>)>) -> (String, Arc<Mutex<Vec<String>>>, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/model", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let capture = requests.clone();
        let thread = std::thread::spawn(move || {
            for (headers, body) in replies {
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    if stream.read(&mut byte).unwrap_or(0) == 0 { break; }
                    request.push(byte[0]);
                }
                capture.lock().unwrap().push(String::from_utf8_lossy(&request).to_lowercase());
                let _ = stream.write_all(headers.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        (url, requests, thread)
    }
    fn seed(dest: &Path, url: &str, bytes: &[u8]) {
        std::fs::write(dest.with_extension("part"), bytes).unwrap();
        std::fs::write(dest.with_extension("part.json"), serde_json::to_vec(&Resume {
            url: url.to_owned(), checksum: "sha".into(), total: 200_000, validator: Some("\"old\"".into()),
        }).unwrap()).unwrap();
    }
    #[test]
    fn resumes_saved_bytes_and_confirms_range_before_appending() {
        let (url, requests, worker) = server(vec![("HTTP/1.1 206 Partial Content\r\nContent-Length: 100000\r\nContent-Range: bytes 100000-199999/200000\r\nETag: \"old\"\r\nConnection: close\r\n\r\n".into(), vec![2;100000])]);
        let dir = tempfile::tempdir().unwrap(); let dest = dir.path().join("model.bin"); seed(&dest, &url, &vec![1;100000]);
        transfer(&url, &dest, "sha", &AtomicBool::new(false), |_,_,_|{}).unwrap();
        worker.join().unwrap();
        let bytes = std::fs::read(dest.with_extension("part")).unwrap();
        assert_eq!(&bytes[..100000], &vec![1;100000]); assert_eq!(&bytes[100000..], &vec![2;100000]);
        assert!(requests.lock().unwrap()[0].contains("range: bytes=100000-"));
        assert!(requests.lock().unwrap()[0].contains("if-range: \"old\"")); assert!(!dest.exists());
    }
    #[test]
    fn changed_or_range_ignoring_server_restarts_without_mixing_bytes() {
        let (url, _, worker) = server(vec![("HTTP/1.1 200 OK\r\nContent-Length: 200000\r\nETag: \"new\"\r\nConnection: close\r\n\r\n".into(), vec![2;200000])]);
        let dir = tempfile::tempdir().unwrap(); let dest = dir.path().join("model.bin"); seed(&dest, &url, &vec![1;100000]);
        transfer(&url, &dest, "sha", &AtomicBool::new(false), |_,_,_|{}).unwrap(); worker.join().unwrap();
        assert_eq!(std::fs::read(dest.with_extension("part")).unwrap(), vec![2;200000]); assert!(!dest.exists());
    }
    #[test]
    fn inconsistent_partial_response_preserves_existing_progress() {
        for headers in [
            "Content-Range: bytes 99999-199999/200000\r\nETag: \"old\"\r\nContent-Length: 100001",
            "Content-Range: bytes 100000-199999/200000\r\nETag: \"new\"\r\nContent-Length: 100000",
            "ETag: \"old\"\r\nContent-Length: 100000",
        ] {
            let (url, _, worker) = server(vec![(format!("HTTP/1.1 206 Partial Content\r\n{headers}\r\nConnection: close\r\n\r\n"), vec![])]);
            let dir = tempfile::tempdir().unwrap(); let dest = dir.path().join("model.bin"); seed(&dest, &url, &vec![1;100000]);
            assert!(transfer(&url, &dest, "sha", &AtomicBool::new(false), |_,_,_|{}).is_err()); worker.join().unwrap();
            assert_eq!(std::fs::read(dest.with_extension("part")).unwrap(), vec![1;100000]); assert!(!dest.exists());
        }
    }
    #[test]
    fn interruption_keeps_bytes_and_second_request_resumes_them() {
        let (url, requests, worker) = server(vec![
            ("HTTP/1.1 200 OK\r\nContent-Length: 200000\r\nETag: \"old\"\r\nConnection: close\r\n\r\n".into(), vec![1;100000]),
            ("HTTP/1.1 206 Partial Content\r\nContent-Length: 100000\r\nContent-Range: bytes 100000-199999/200000\r\nETag: \"old\"\r\nConnection: close\r\n\r\n".into(), vec![2;100000]),
        ]);
        let dir = tempfile::tempdir().unwrap(); let dest = dir.path().join("model.bin"); let cancel = AtomicBool::new(false);
        assert!(transfer(&url, &dest, "sha", &cancel, |_,_,_|{}).is_err());
        assert_eq!(std::fs::metadata(dest.with_extension("part")).unwrap().len(), 100000);
        transfer(&url, &dest, "sha", &cancel, |_,_,_|{}).unwrap(); worker.join().unwrap();
        assert!(requests.lock().unwrap()[1].contains("range: bytes=100000-")); assert!(!dest.exists());
    }
    #[test]
    fn cancellation_keeps_saved_bytes_and_never_publishes() {
        let (url, _, worker) = server(vec![("HTTP/1.1 200 OK\r\nContent-Length: 200000\r\nETag: \"old\"\r\nConnection: close\r\n\r\n".into(), vec![2;200000])]);
        let dir = tempfile::tempdir().unwrap(); let dest = dir.path().join("model.bin"); let cancel = AtomicBool::new(false);
        let result = transfer(&url, &dest, "sha", &cancel, |done,_,_| { if done > 0 { cancel.store(true, Ordering::Release); } });
        assert!(result.unwrap_err().to_string().contains("paused")); worker.join().unwrap();
        assert!(std::fs::metadata(dest.with_extension("part")).unwrap().len() > 0); assert!(!dest.exists());
    }
    #[test]
    fn cancellation_interrupts_a_stalled_response_without_waiting_for_read_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/model", listener.local_addr().unwrap());
        let (sent, received) = std::sync::mpsc::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0;1024]; let _ = stream.read(&mut request);
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 200000\r\nETag: \"old\"\r\n\r\n").unwrap();
            sent.send(()).unwrap();
            let _ = wait.recv_timeout(Duration::from_secs(3));
        });
        let cancel = Arc::new(AtomicBool::new(false)); let flag = cancel.clone();
        let canceller = std::thread::spawn(move || {
            received.recv().unwrap(); std::thread::sleep(Duration::from_millis(50)); flag.store(true, Ordering::Release);
        });
        let dir = tempfile::tempdir().unwrap(); let dest = dir.path().join("model.bin");
        let start = std::time::Instant::now();
        assert!(transfer(&url, &dest, "sha", &cancel, |_,_,_|{}).unwrap_err().to_string().contains("paused"));
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(!dest.exists()); release.send(()).unwrap(); server.join().unwrap(); canceller.join().unwrap();
    }
    #[test]
    fn last_modified_can_resume_when_no_strong_etag_is_available() {
        let (url, requests, worker) = server(vec![("HTTP/1.1 206 Partial Content\r\nContent-Length: 100000\r\nContent-Range: bytes 100000-199999/200000\r\nETag: W/\"weak\"\r\nLast-Modified: Wed, 21 Oct 2026 09:00:00 GMT\r\nConnection: close\r\n\r\n".into(), vec![2;100000])]);
        let dir = tempfile::tempdir().unwrap(); let dest = dir.path().join("model.bin");
        std::fs::write(dest.with_extension("part"), vec![1;100000]).unwrap();
        std::fs::write(dest.with_extension("part.json"), serde_json::to_vec(&Resume {
            url: url.clone(), checksum: "sha".into(), total: 200000,
            validator: Some("Wed, 21 Oct 2026 09:00:00 GMT".into()),
        }).unwrap()).unwrap();
        transfer(&url, &dest, "sha", &AtomicBool::new(false), |_,_,_|{}).unwrap(); worker.join().unwrap();
        assert!(requests.lock().unwrap()[0].contains("if-range: wed, 21 oct 2026 09:00:00 gmt"));
        assert_eq!(std::fs::metadata(dest.with_extension("part")).unwrap().len(), 200000);
    }
    #[test]
    fn untrusted_legacy_partial_and_changed_checksum_start_fresh() {
        for old_checksum in [None, Some("different-checksum")] {
            let (url, requests, worker) = server(vec![("HTTP/1.1 200 OK\r\nContent-Length: 200000\r\nETag: \"old\"\r\nConnection: close\r\n\r\n".into(), vec![2;200000])]);
            let dir = tempfile::tempdir().unwrap(); let dest = dir.path().join("model.bin");
            std::fs::write(dest.with_extension("part"), vec![1;100000]).unwrap();
            if let Some(checksum) = old_checksum {
                std::fs::write(dest.with_extension("part.json"), serde_json::to_vec(&Resume {
                    url: url.clone(), checksum: checksum.into(), total: 200000, validator: Some("\"old\"".into()),
                }).unwrap()).unwrap();
            }
            transfer(&url, &dest, "sha", &AtomicBool::new(false), |_,_,_|{}).unwrap(); worker.join().unwrap();
            assert!(!requests.lock().unwrap()[0].contains("range:"));
            assert_eq!(std::fs::read(dest.with_extension("part")).unwrap(), vec![2;200000]);
        }
    }
    #[test]
    fn disk_full_budget_refuses_before_overwriting_saved_progress() {
        assert!(check_space(500, 1000).unwrap_err().to_string().contains("space"));
        assert!(check_space(2_000_000, 1000).is_ok());
    }
}
