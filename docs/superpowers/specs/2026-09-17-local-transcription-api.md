# Local transcription API — Lectus

**Date:** 2026-09-17
**Status:** specified, not implemented
**Depends on:** the Organic token gate (`docs/superpowers/specs/2026-09-17-organic-token-gate.md`)

## What this is

Lectus can already put text into any focused field. What it cannot do is answer a
question: *here is some audio, what does it say?* Other apps — Organic first —
want that answer, not keystrokes.

So: a loopback HTTP server inside Lectus with one working endpoint,
`POST /v1/transcribe`. Audio in, text out.

**This is a microphone behind an HTTP door, and the door faces the browser.**
Any page the user has open can issue a cross-origin request to `127.0.0.1` —
that is allowed by design in every browser. An unauthenticated endpoint here is
not "a local convenience"; it is a microphone that `ads.example.com` can reach.
Every rule below exists because of that one sentence.

The single most important mitigation is architectural, not cryptographic:
**v1 never opens the microphone.** The API transcribes audio the caller already
holds. A caller that wants a recording must get it from the user through the
browser's own `getUserMedia` consent, in its own origin, with its own permission
prompt. Lectus's mic stays bound to the user's physical hotkey.

## What v1 does and does not do

| | |
|---|---|
| Does | Transcribe a WAV the caller uploads; apply the user's dictionary and replacement rules; return text + detected language |
| Does not | Open the mic, start/stop recording, stream partials, inject text, read history, read or write config, reach the network |

That list is the abuse ceiling. A fully compromised allowlisted origin gets
transcripts of audio it already had, plus some of the user's GPU. It does not get
the room.

## The rules

| Thing | Value |
|---|---|
| Bind | `127.0.0.1` only. Never `0.0.0.0`, never a LAN option, not ever |
| Port | fixed, default **`8756`**, configurable. Not port 0 — a caller must be able to find it |
| Enabled | **off by default**; explicit toggle in Settings → API |
| Auth | `Authorization: Bearer <token>`, per install, required on every route |
| Token | 32 random bytes, base58, generated on first enable, stored in `api.json` |
| Origins | exact-string allowlist. No wildcard, no suffix match, no regex |
| Body | `Content-Type: audio/wav`, 16 kHz mono WAV, ≤ **8 MB** and ≤ **120 s** |
| Concurrency | 1 API job at a time; a running dictation always wins |
| Rate | 30 requests/minute, then `429` |
| Gate | the Organic gate is checked per call, same as dictation |

### Why a fixed port, when the wallet link uses port 0

`connect.rs` can use port 0 because Lectus *tells* the browser the URL — it opens
the tab itself. Here the caller starts the conversation and has no way to learn a
random port (a web page cannot read `api.json`). So: a fixed default, editable in
Settings. If the port is taken, the API **fails to start and says so in Settings** —
it must not silently move somewhere the caller cannot find.

## The endpoints

### `POST /v1/transcribe`

Request — raw WAV bytes as the body:

```
POST /v1/transcribe HTTP/1.1
Host: 127.0.0.1:8756
Authorization: Bearer <token>
Content-Type: audio/wav
X-Lectus-Language: pt        (optional; ISO code or "auto", default = user's setting)
X-Lectus-Cleanup: 0          (optional; "1" runs AI cleanup, honoured only if the user enabled it)
```

Response `200`:

```json
{ "text": "isto é um teste", "language": "pt", "duration_ms": 1640, "transcribe_ms": 295, "engine": "local" }
```

Options ride in headers, not a JSON envelope, so the body stays raw audio: no
base64 (33% bigger, and it would double-buffer a multi-megabyte upload), no
multipart parser to write and harden.

### `GET /v1/health`

Token required. Returns `{"ok":true,"version":"0.6.0","busy":false}`. Callers use
it to decide whether to show a "dictate with Lectus" button.

Presence is not a secret worth defending: any page can already distinguish
*connection refused* from *401*, so hiding behind an unauthenticated health check
would buy nothing. Requiring the token here keeps the version string and busy
state from leaking to unauthorized pages.

### `OPTIONS /v1/*`

CORS preflight. Answered only for an allowlisted `Origin`.

### Errors

Every failure is JSON: `{"error":{"code":"...","message":"..."}}`.

| Code | HTTP | When |
|---|---|---|
| `unauthorized` | 401 | missing, malformed, or wrong token |
| `forbidden_origin` | 403 | `Origin` present and not allowlisted |
| `locked` | 403 | Organic gate refuses (`Locked` / `Unlinked`) |
| `unsupported_media` | 415 | `Content-Type` is not `audio/wav` |
| `bad_audio` | 400 | not a WAV, not mono, not 16 kHz, or empty |
| `payload_too_large` | 413 | over 8 MB or over 120 s |
| `busy` | 503 | a dictation or another API job is running |
| `rate_limited` | 429 | over 30 requests/minute |
| `engine_unavailable` | 503 | no model loaded, or the cloud key is missing |

## Threat model

| Attacker | What they try | What stops it |
|---|---|---|
| Any web page the user has open | `fetch('http://127.0.0.1:8756/v1/transcribe', …)` | Bearer token; `Origin` allowlist; **auth is checked before any work** — an unauthorized request never reaches the decoder or the GPU |
| The same page, dodging preflight | Send `Content-Type: text/plain` so the browser skips `OPTIONS` | We require `audio/wav` (not a CORS-safelisted type) **and** an `Authorization` header — either one forces a preflight, which an unlisted origin fails |
| DNS rebinding | `evil.com` resolves to `127.0.0.1`, so the request looks same-origin to the browser | `Origin` is still `https://evil.com` → denied. Belt and braces: reject any request whose `Host` is not `127.0.0.1`/`localhost` on our port |
| Another machine on the LAN | Connect to port 8756 | Never bound off loopback |
| Malware already running as the user | Read `api.json`, call the API | Accepted. It could equally read `history.json` or keylog. The token is not a defence against code already running as the user |
| Organic's origin compromised (XSS, bad deploy) | Call the API with the user's token | Bounded by design: it gets transcripts of audio **it supplies** and some GPU time. No mic, no history, no config, no injection |
| Anyone with the token | Flood it | 1 concurrent job, 30/min, 8 MB, 120 s, and dictation preempts |
| Shoulder-surfer / screenshot | Read the token off the Settings screen | Masked by default, reveal-on-click, one-click regenerate |

### Things deliberately not done

- **No `Access-Control-Allow-Credentials`.** The token travels in a header. Turning
  on credentials would make ambient cookies matter and open a CSRF shape that does
  not otherwise exist.
- **No wildcard origin, not even for `GET`.** `*` here means "every page on the
  internet", and the endpoint is a transcriber on the user's hardware.
- **No window popping.** A refused call returns JSON. It does not raise the Home
  window — a remote caller must not be able to make the user's UI jump.
- **No transcript text in the log.** Match the existing pipeline: log origin, byte
  count, and milliseconds; never the words.

## The flow

1. **Enable.** Settings → API, toggle on. Lectus generates a token (32 bytes from
   `getrandom`, base58 via `bs58` — both already dependencies), writes `api.json`,
   binds `127.0.0.1:8756`, starts the server thread.
2. **Allow an origin.** The user adds `https://<organic-domain>` to the allowlist.
   Empty allowlist = browsers get nothing; `curl` still works (no `Origin` header).
3. **Copy the token** into Organic, once.
4. **Call.** Organic records with `getUserMedia`, decodes to 16 kHz mono WAV, POSTs.
5. **Serve.** The server checks, in this order and before touching the audio:
   `Host` → `Origin` → token → gate → rate limit → busy → `Content-Type` → size →
   WAV shape. Then it decodes, runs the job on `TranscribeWorker`, applies the
   dictionary and rules, and replies.
6. **Show it.** While an API job runs, the pill shows the API indicator. Settings
   shows last call: time, origin, duration.

## Visibility

Off-by-default is only half of consent; the other half is knowing when it is on.

- **Pill:** a small persistent dot/badge whenever the API is enabled, and a
  distinct pulse while an API transcription runs. Not the recording animation —
  nothing is being recorded, and implying otherwise trains the user to ignore it.
- **Tray tooltip:** "Lectus — API on (port 8756)".
- **Settings → API:** enabled state, port, masked token, allowlist, and the last
  five calls (time, origin, audio length, result length).
- **State events:** a new `api-activity` event. Do **not** reuse `state-changed`
  `recording`/`transcribing` — the existing pill states mean the user's own mic.

## Interaction with dictation

The whisper engine is one thread (`worker::TranscribeWorker`). Sharing it is
correct — two whisper contexts would double VRAM — but the priority must be
explicit:

- If `RecordingState != Idle`, an API call is rejected **immediately** with `busy`.
  It does not queue. The user's own dictation must never wait behind a web page.
- Once an API job is on the worker, a dictation starting during it waits for the
  worker as it already does today. API jobs are capped at 120 s of audio partly to
  bound that wait.
- Capture, injection, and the hotkey pipeline are **not touched**. The API path
  shares only the transcription engine and the post-processing functions.

## Post-processing

Applied: `apply_dictionary_spellings`, then `apply_rules`, then the same trim as
dictation. The user's vocabulary is most of the quality, and Organic should
benefit from it. `bias_prompt` is fed to the engine as `initial_prompt`, same as
dictation.

AI cleanup is **off** for API calls unless the caller asks (`X-Lectus-Cleanup: 1`)
*and* the user has cleanup enabled. It costs 111–166 ms locally and can spawn the
Claude CLI; a remote caller should not trigger process spawns by default.

History: API transcripts are **not** written to `history.json` in v1. History is
the user's own dictation record and it feeds vocabulary work; another app's audio
does not belong there. One config flag (`api_log_to_history`, default false) if
that turns out wrong.

## Where it lives

```
src-tauri/src/api/
  mod.rs      ApiState, enable/disable, config plumbing, the call log
  server.rs   tiny_http loop, routing, Host/Origin/CORS, size caps  (models connect.rs)
  token.rs    generate, load/save api.json, constant-time compare
  audio.rs    WAV bytes -> Vec<f32>, shape validation (hound, in memory)
```

Config additions (`config.rs`): `api_enabled: bool` (false), `api_port: u16`
(8756), `api_allowed_origins: Vec<String>` (empty).

The **token lives in `api.json`**, next to `license.json` in
`%APPDATA%\ai.organic.lectus\` — never in `config.json`, for the same reason the
license is not: `config.json` is the file a user pastes into an issue or syncs.
Same atomic-rename write discipline as `license::save_to`.

Frontend: a Settings → API tab, and the pill indicator.

Reused, nothing new added to `Cargo.toml`: `tiny_http`, `bs58`, `getrandom`,
`hound`, `serde_json`, `worker::TranscribeWorker`.

## Tests

`cargo test --release --lib` (Vulkan recipe env), mirroring the raw-socket test
helpers already in `connect.rs`:

1. Token compare is constant-time and rejects wrong length, wrong value, missing
   and malformed `Authorization` headers.
2. Origin matcher: exact match passes; differing scheme, port, subdomain, trailing
   slash, and case-mangled host all fail; empty allowlist blocks every browser origin.
3. `Host` header check rejects a rebinding-shaped host.
4. Preflight answers only allowlisted origins and never sends `Allow-Credentials`.
5. Caps: oversized body, wrong `Content-Type`, non-WAV, stereo, and 44.1 kHz all
   rejected — and rejected *without* decoding.
6. Ordering: an unauthorized request produces no engine call (assert with a stub).
7. `busy` while `RecordingState != Idle`; `locked` when the gate refuses.
8. Round trip on a fixture WAV from `tests/audio_samples/` through a stub engine.

## Client contract

```bash
curl -s -X POST http://127.0.0.1:8756/v1/transcribe \
  -H "Authorization: Bearer $LECTUS_TOKEN" \
  -H "Content-Type: audio/wav" \
  --data-binary @clip.wav
```

Browser callers must convert first — `MediaRecorder` gives webm/opus, which v1
does not decode. `decodeAudioData` → `OfflineAudioContext(1, …, 16000)` →
`startRendering()` → write a 16-bit PCM WAV header. Roughly 30 lines, no
dependency. Documenting that snippet in Organic's repo is part of shipping this.

## Open decisions

1. **Organic's origins.** Production domain, preview deploys (`*.vercel.app` is a
   wildcard and is therefore out — list each one), and `http://localhost:3000` for
   local development. Needs the real list before the allowlist ships with a default.
2. **Port 8756** is arbitrary and unregistered. Fine, but check it against nothing
   common on the user's machines before hardcoding the default.
3. **Codec support.** WAV-only pushes conversion onto every caller. Accepting
   webm/opus means an audio decoder in the process — new dependency, new parser on
   an attack surface. Deferred, deliberately.
4. **v2 candidates, each needing its own threat pass:** SSE partials; mic control
   behind a per-origin, in-app consent prompt with a visible recording indicator the
   web page cannot suppress; a read-only history endpoint.
