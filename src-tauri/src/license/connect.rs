//! The wallet hand-off.
//!
//! A Tauri webview cannot see browser extensions, so Phantom and friends are
//! unreachable from inside the app. Instead the app serves a single page on
//! `127.0.0.1:<random port>` and opens it in the real browser: wallets treat
//! localhost as a secure origin, so the injected provider works there.
//!
//! The server accepts exactly one successful link and then shuts down. It
//! binds the loopback interface only, and it never receives a private key —
//! the browser sends back a public key and a signature over our nonce.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// How long the page stays open before the server gives up.
const LINK_TIMEOUT: Duration = Duration::from_secs(300);

/// What the browser posts back once the wallet has signed.
#[derive(Debug, Deserialize)]
pub struct LinkProof {
    pub pubkey: String,
    pub signature: String,
}

/// A running link attempt: the URL to open, and the channel the proof arrives on.
pub struct LinkSession {
    pub url: String,
    pub proof: mpsc::Receiver<Result<LinkProof>>,
}

/// Serve the connect page and wait (on a background thread) for one signature.
pub fn start(nonce: &str) -> Result<LinkSession> {
    // Port 0: let the OS pick a free one. Loopback only — nothing on the LAN
    // can reach this.
    let server = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|e| anyhow!("could not start the local link server: {e}"))?;
    let port = server
        .server_addr()
        .to_ip()
        .ok_or_else(|| anyhow!("local link server has no IP address"))?
        .port();

    let page = page_html(nonce);
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let deadline = Instant::now() + LINK_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                let _ = tx.send(Err(anyhow!("wallet link timed out")));
                return;
            }
            let Ok(Some(mut request)) = server.recv_timeout(remaining) else {
                continue;
            };

            match (request.method().as_str(), request.url()) {
                ("POST", url) if url.starts_with("/link") => {
                    let mut body = String::new();
                    let read = std::io::Read::read_to_string(request.as_reader(), &mut body);
                    let parsed = read
                        .map_err(|e| anyhow!("could not read the link request: {e}"))
                        .and_then(|_| {
                            serde_json::from_str::<LinkProof>(&body)
                                .map_err(|e| anyhow!("malformed link request: {e}"))
                        });

                    let ok = parsed.is_ok();
                    let _ = tx.send(parsed);
                    let _ = request.respond(json_response(
                        if ok { 200 } else { 400 },
                        if ok { r#"{"ok":true}"# } else { r#"{"ok":false}"# },
                    ));
                    // One link per session: stop listening either way, so a
                    // stale tab cannot re-link this app later.
                    return;
                }
                _ => {
                    let _ = request.respond(html_response(&page));
                }
            }
        }
    });

    Ok(LinkSession {
        url: format!("http://127.0.0.1:{port}/"),
        proof: rx,
    })
}

/// Open the link page in the user's default browser.
///
/// The system browser is the point: it is where the wallet extension lives.
pub fn open_in_browser(url: &str) -> Result<()> {
    use std::process::Command;
    #[cfg(target_os = "windows")]
    // The empty string is `start`'s title argument — without it, a quoted URL
    // is taken as the window title and nothing opens.
    let mut command = {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut command = {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };

    command
        .spawn()
        .map_err(|e| anyhow!("could not open the browser: {e}"))?;
    Ok(())
}

fn html_response(body: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let mut response = tiny_http::Response::from_string(body);
    response.add_header(
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
            .expect("static header"),
    );
    response
}

fn json_response(status: u16, body: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let mut response =
        tiny_http::Response::from_string(body).with_status_code(tiny_http::StatusCode(status));
    response.add_header(
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
            .expect("static header"),
    );
    response
}

/// The connect page, with this session's nonce baked in.
///
/// Deliberately dependency-free: no CDN, no wallet adapter bundle. It talks to
/// the injected provider directly and base58-encodes the signature itself, so
/// the page works on a flaky connection and ships nothing we did not write.
pub fn page_html(nonce: &str) -> String {
    let message = super::verify::challenge_message(nonce);
    PAGE_TEMPLATE
        .replace("__NONCE__", &js_string(nonce))
        .replace("__MESSAGE__", &js_string(&message))
}

/// Escape a value for embedding inside a JS single-quoted string literal.
fn js_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "")
        .replace('<', "\\x3c")
}

const PAGE_TEMPLATE: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Link your wallet — Lectus</title>
<style>
  :root { color-scheme: light dark; --bg:#f6f5f2; --fg:#14140f; --muted:#6b6b60;
          --card:#fffffe; --line:#e3e1d9; --accent:#2f7a4d; }
  @media (prefers-color-scheme: dark) {
    :root { --bg:#111310; --fg:#f2f2ec; --muted:#9a9a8d; --card:#1a1c19; --line:#2c2f2a; --accent:#5bbd82; }
  }
  * { box-sizing: border-box; }
  body { margin:0; min-height:100vh; display:grid; place-items:center; padding:24px;
         background:var(--bg); color:var(--fg);
         font:16px/1.55 ui-sans-serif,-apple-system,"Segoe UI",Roboto,sans-serif; }
  .card { width:100%; max-width:460px; background:var(--card); border:1px solid var(--line);
          border-radius:16px; padding:32px; }
  h1 { margin:0 0 6px; font-size:22px; letter-spacing:-0.01em; }
  p { margin:0 0 18px; color:var(--muted); }
  button { width:100%; padding:13px 18px; margin-bottom:10px; font-size:15px; font-weight:600;
           color:#fff; background:var(--accent); border:0; border-radius:10px; cursor:pointer; }
  button:disabled { opacity:.55; cursor:default; }
  button.secondary { color:var(--fg); background:transparent; border:1px solid var(--line); font-weight:500; }
  .note { font-size:13px; color:var(--muted); margin:14px 0 0; }
  .status { margin-top:16px; padding:12px 14px; border-radius:10px; font-size:14px; display:none; }
  .status.show { display:block; }
  .status.err { background:rgba(190,60,60,.12); color:#c0392b; }
  .status.ok  { background:rgba(47,122,77,.12); color:var(--accent); }
</style>
</head>
<body>
<div class="card">
  <h1>Link your wallet</h1>
  <p>Lectus checks that this wallet holds ORGANIC or Mycel. You will sign a
     message — it moves no SOL, no tokens, and approves nothing.</p>
  <div id="wallets"></div>
  <p class="note">Nothing is sent anywhere but your own machine: this page is served
     by Lectus on localhost.</p>
  <div id="status" class="status"></div>
</div>
<script>
const NONCE = '__NONCE__';
const MESSAGE = '__MESSAGE__';
const ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

function base58(bytes) {
  const digits = [0];
  for (const byte of bytes) {
    let carry = byte;
    for (let i = 0; i < digits.length; i++) {
      carry += digits[i] << 8;
      digits[i] = carry % 58;
      carry = (carry / 58) | 0;
    }
    while (carry > 0) { digits.push(carry % 58); carry = (carry / 58) | 0; }
  }
  let out = '';
  for (const byte of bytes) { if (byte === 0) out += '1'; else break; }
  for (let i = digits.length - 1; i >= 0; i--) out += ALPHABET[digits[i]];
  return out;
}

function show(kind, text) {
  const el = document.getElementById('status');
  el.className = 'status show ' + kind;
  el.textContent = text;
}

// Injected providers, in the order we offer them. A wallet that is not
// installed is simply not listed.
function providers() {
  const found = [];
  const phantom = window.phantom && window.phantom.solana;
  if (phantom && phantom.isPhantom) found.push({ name: 'Phantom', provider: phantom });
  if (window.solflare && window.solflare.isSolflare) found.push({ name: 'Solflare', provider: window.solflare });
  const backpack = window.backpack && window.backpack.solana ? window.backpack.solana : window.backpack;
  if (backpack && backpack.isBackpack) found.push({ name: 'Backpack', provider: backpack });
  if (!found.length && window.solana) found.push({ name: 'your Solana wallet', provider: window.solana });
  return found;
}

async function link(provider, button) {
  button.disabled = true;
  try {
    show('ok', 'Waiting for your wallet…');
    const connection = await provider.connect();
    const pubkey = (connection && connection.publicKey ? connection.publicKey : provider.publicKey).toString();
    const signed = await provider.signMessage(new TextEncoder().encode(MESSAGE), 'utf8');
    const signature = base58(signed.signature || signed);

    const response = await fetch('/link', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pubkey: pubkey, signature: signature, nonce: NONCE })
    });
    if (!response.ok) throw new Error('Lectus rejected the signature.');
    show('ok', 'Wallet linked. You can close this tab and go back to Lectus.');
    document.getElementById('wallets').innerHTML = '';
  } catch (error) {
    show('err', (error && error.message) ? error.message : 'Could not link this wallet.');
    button.disabled = false;
  }
}

const list = document.getElementById('wallets');
const found = providers();
if (!found.length) {
  show('err', 'No Solana wallet found in this browser. Install Phantom, then reload this page.');
} else {
  found.forEach(function (entry, index) {
    const button = document.createElement('button');
    button.textContent = 'Connect ' + entry.name;
    if (index > 0) button.className = 'secondary';
    button.onclick = function () { link(entry.provider, button); };
    list.appendChild(button);
  });
}
</script>
</body>
</html>
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_carries_the_nonce_and_message() {
        let html = page_html("NonceABC123");
        assert!(html.contains("NonceABC123"));
        // The signed text is embedded as one JS literal, newlines escaped.
        assert!(html.contains("const MESSAGE = 'Lectus"));
        assert!(!html.contains("__NONCE__"));
        assert!(!html.contains("__MESSAGE__"));
    }

    #[test]
    fn js_string_escapes_quote_backslash_newline_and_tag() {
        assert_eq!(js_string("it's"), "it\\'s");
        assert_eq!(js_string("a\\b"), "a\\\\b");
        assert_eq!(js_string("a\r\nb"), "a\\nb");
        // A literal </script> inside the nonce must not close the tag.
        assert_eq!(js_string("</script>"), "\\x3c/script>");
    }

    #[test]
    fn server_binds_loopback_and_hands_back_its_url() {
        let session = start("NonceABC123").unwrap();
        assert!(session.url.starts_with("http://127.0.0.1:"));
        assert!(!session.url.ends_with(":0/"));
    }

    #[test]
    fn server_serves_the_page_and_accepts_one_proof() {
        let session = start("NonceABC123").unwrap();

        let page = ureq_get(&session.url).expect("page should be served");
        assert!(page.contains("Link your wallet"));

        let status = ureq_post(
            &format!("{}link", session.url),
            r#"{"pubkey":"PK","signature":"SIG"}"#,
        )
        .expect("link should be accepted");
        assert_eq!(status, 200);

        let proof = session
            .proof
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("proof should arrive")
            .expect("proof should parse");
        assert_eq!(proof.pubkey, "PK");
        assert_eq!(proof.signature, "SIG");
    }

    // Minimal HTTP client so the test needs no extra dependency.
    fn ureq_get(url: &str) -> Option<String> {
        raw_request(url, "GET", None)
    }

    fn ureq_post(url: &str, body: &str) -> Option<u16> {
        let response = raw_request(url, "POST", Some(body))?;
        response
            .lines()
            .next()?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    }

    fn raw_request(url: &str, method: &str, body: Option<&str>) -> Option<String> {
        use std::io::{Read, Write};
        let rest = url.strip_prefix("http://")?;
        let (authority, path) = rest.split_once('/')?;
        let mut stream = std::net::TcpStream::connect(authority).ok()?;
        let body = body.unwrap_or("");
        let request = format!(
            "{method} /{path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).ok()?;
        let mut response = String::new();
        stream.read_to_string(&mut response).ok()?;
        Some(response)
    }
}
