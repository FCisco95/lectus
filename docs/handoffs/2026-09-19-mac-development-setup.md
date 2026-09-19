# Lectus — Mac Development Setup (2026-09-19)

## TL;DR

Lectus has been cloned and prepared for macOS development. JavaScript dependencies,
the fallback Whisper model, and native Rust dependencies are installed. The frontend
production build passes. No product source files were changed.

## Setup completed

- Repository: clean `master` tracking `origin/master` at `dfff2cb`.
- Ran `npm ci` and `npm run build` successfully.
- Ran `scripts/download_model.sh`; it placed the gitignored 74 MB fallback model
  at `models/ggml-tiny.en.bin`.
- Baseline toolchain verified: Node 24.14.0, npm 11.19.1, Rust 1.94.0, and Xcode
  Command Line Tools.
- Production dependency audit reports zero vulnerabilities.

## Verification note

`cargo test --release --lib` compiles but produces 111 passing tests, 4 intentional
ignores, and two failures in `src-tauri/src/autostart.rs`. They exercise Windows
paths and use `PathBuf::file_name()`; macOS does not treat `\\` as a path separator,
so the tests do not reach their intended Windows branch. This is a platform-test
portability issue, not an environment failure. No source changes were made.

## What to do next

The committed product direction is to spec the local transcription API before coding:
`POST /v1/transcribe`, a per-install token, explicit Organic origin allowlist, off by
default with clear activity state, and no remote microphone control in v1. See the
canonical state in `docs/HANDOFF.md`.

## Suggested skills

- `handoff-memory`

## Generated artifacts this session

| What | Where it lives | Notes |
|---|---|---|
| Local fallback Whisper model | `models/ggml-tiny.en.bin` | Gitignored; recreate with `scripts/download_model.sh` on a new machine. |

## Next-session prompt

```
Lectus is locally set up on macOS; frontend build passes and the fallback Whisper model is present. Continue with the local transcription API design, not the capture, injection, or shortcut pipeline.

Files: docs/HANDOFF.md, docs/superpowers/specs/2026-09-17-organic-token-gate.md, src-tauri/src/autostart.rs
Model: Codex Sonnet 5 — focused Rust/Tauri product planning and implementation.
Skills: handoff-memory

Write the API spec for `POST /v1/transcribe` before implementation, preserving the existing security constraints.
```
