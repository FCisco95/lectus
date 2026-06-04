# Lectus Handoff — 2026-06-02 — Phase 3 done, transcription too slow

_Snapshot. Canonical living doc: `docs/HANDOFF.md`._

## TL;DR

Phase 3 ("WhisperFlow UX", v0.4.0) fully implemented on branch
`feat/phase3-whisperflow-ux` (M0–M7, 42 tests pass, installer builds). Device
testing found ONE blocker: **local whisper transcription takes ~16–30 s for ~2 s
of audio** — unusably slow. Audio, accuracy ("Testing 1, 2, 3"), language
auto-detect, and injection all verified working. Slowness is the only real bug;
"didn't paste" is a side effect (focus moves during the 16 s wait).

## Next task

Make transcription fast (< 2 s). Likely root cause: **whisper.cpp built without
AVX2/FMA SIMD** by `whisper-rs`. Options: enable SIMD in the build; use a GPU
backend (vulkan/cuda); or default to the faster `ggml-tiny.bin` multilingual
model. Measure via the `%TEMP%\lectus-diag.log` "transcribed in Xs" line.

## Fixes made this session (uncommitted)

- **Deadlock fixed**: `src-tauri/src/worker.rs` — single dedicated whisper thread
  (was freezing on 2nd dictation due to ggml/OpenMP cross-thread deadlock). KEEP.
- **n_threads**: `transcription/local.rs` `set_n_threads(min(8,cores))` — partial
  (30 s → 17 s). KEEP.
- **Temporary diagnostics**: `diag()` in `lib.rs`, device logging in
  `audio/mod.rs` → `%TEMP%\lectus-diag.log`. REMOVE before final commit.

## Gotchas

- Installer exe is `chirp.exe` (legacy name) at `%LOCALAPPDATA%\Lectus\`.
- **Kill `chirp.exe` before `cargo build --release`** or the build silently fails
  to relink (exe locked → "Access is denied", masked by `| tail`).
- Release build = GUI subsystem, **no console** → `eprintln!` is dropped; use the
  file-based `diag()`.
- Model/config shared by dev + installed (`%APPDATA%\ai.organic.lectus\`).

## Suggested skills

`superpowers:systematic-debugging`, `context7`/`microsoft-docs:microsoft-code-reference`
(whisper-rs build flags), `run`/`verify`, then `commit-push-pr`.
