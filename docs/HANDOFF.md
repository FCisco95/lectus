# Lectus — HANDOFF

_Last updated: 2026-06-02 • Branch: `feat/phase3-whisperflow-ux` (uncommitted changes) • Version: 0.4.0_

## TL;DR

This session **debunked the SIMD hypothesis** from last session and implemented two perf fixes + a new UX feature:

1. **WhisperState pre-creation** — `create_state()` used to run per-dictation, allocating ~200-400 MB of compute buffers every time. It now runs once at engine load. This eliminates a major source of latency.
2. **Default model → `ggml-tiny.bin`** — 3-4× less computation than `ggml-base` on CPU; multilingual; still supports Portuguese.
3. **Model picker in Settings** — new "Models" tab (🧠) lets the user download and switch between Tiny / Base / Small models. Hot-swaps the engine without restart.
4. **Debug scaffolding removed** — `diag()` in `lib.rs` and device-log block in `audio/mod.rs` cleaned up.

**Build passes** (`cargo build --release` → clean in ~66 s; `npm run build` → clean). Binary at `src-tauri/target/release/chirp.exe` is fresh (2026-06-02 16:30).

**Still unverified on-device** — need a real dictation test to confirm speed.

## What to do next (priority order)

1. **Kill any running `chirp.exe`, run `src-tauri\target\release\chirp.exe`**, dictate "Testing 1, 2, 3". Target: transcription in < 5 s with tiny model. Confirm paste lands in the focused field.
2. **Test the Models tab** — Settings → Models → download Tiny (if needed) → "Use this model" → confirm "Active" badge switches.
3. If transcription is still > 5 s: the remaining bottleneck is likely ggml's CPU thread pool not parallelising well without OpenMP. **The next move is enabling `openmp`** — now safe because all whisper calls are serialised on the `whisper-worker` thread. See "Next perf step" below.
4. If everything looks good: **commit and PR** via `superpowers:finishing-a-development-branch`.

## Root cause finding (important context)

The previous handoff claimed "AVX2/FMA not compiled in". **This was wrong.** Inspection of the actual MSVC build log (`CL.command.1.tlog`) confirmed `/arch:AVX2 /D GGML_AVX2 /D GGML_FMA /D GGML_F16C` are already present — whisper.cpp 1.8.3 auto-detects via `FindSIMD.cmake` when `GGML_NATIVE=ON` (the default on non-cross-compile builds).

The real culprit was **per-call `create_state()`**: every dictation called `ctx.create_state()` which builds 4 compute scheduler graphs (conv / encoder / cross / decoder) + KV caches, allocating and zeroing ~200-400 MB on every call. Now pre-created once in `LocalWhisper::new()`.

## Files changed this session

| File | Change |
|------|--------|
| `src-tauri/src/transcription/local.rs` | Pre-create `WhisperState` in `new()`; `transcribe` now `&mut self`; removed per-call `create_state()` |
| `src-tauri/src/transcription/mod.rs` | Removed unused `TranscriptionEngine` trait impls (caused `&mut self` type mismatch) |
| `src-tauri/src/transcription/model.rs` | Added `ModelInfo`, `AVAILABLE_MODELS` (tiny/base/small); dynamic URL derived from model_name |
| `src-tauri/src/config.rs` | Default `model_name`: `"ggml-base.bin"` → `"ggml-tiny.bin"` |
| `src-tauri/src/lib.rs` | Removed `diag()` + all call sites; `as_ref()→as_mut()` for local transcribe; added `get_models_status`, `download_model`, `select_model` commands; `ModelStatus` serde struct |
| `src-tauri/src/audio/mod.rs` | Removed debug device-log block |
| `src/components/settings/ModelsPanel.tsx` | **New** — model picker with per-model download progress + hot-swap |
| `src/components/settings/LanguagePanel.tsx` | Removed model download section (moved to Models tab) |
| `src/components/Settings.tsx` | Added "Models" tab (`🧠`) |
| `src/styles/settings.css` | Added `.model-card`, `.model-card-active`, `.model-list`, `.model-card-*` + dark mode |

## How to build & test

```powershell
# Always kill first — running exe locks the binary
Stop-Process -Name chirp -Force -ErrorAction SilentlyContinue

cd src-tauri
cargo build --release
.\target\release\chirp.exe
```

The model at `%APPDATA%\ai.organic.lectus\models\ggml-base.bin` is still present. The new default is `ggml-tiny.bin` — if tiny isn't yet downloaded, the app falls back to the bundled `ggml-tiny.en.bin` and the Models tab shows a Download button for Tiny.

## Next perf step if still slow: enable OpenMP

```toml
# src-tauri/Cargo.toml
whisper-rs = { version = "0.16", features = ["openmp"] }
```

Safe now because all `whisper_full` calls are serialised on `whisper-worker` (the cross-thread OpenMP deadlock only happened with `spawn_blocking`'s random thread pool — that's gone). Run `cargo build --release`; whisper.cpp will recompile with MSVC `/openmp`.

## GPU path (Vulkan) — user has a discrete GPU

The user confirmed a discrete GPU. Vulkan backend would cut even the base model to < 1 s. Implementation requirements:
1. `whisper-rs = { version = "0.16", features = ["vulkan"] }` in `Cargo.toml`
2. Vulkan SDK installed (`VULKAN_SDK` env var set); `%VULKAN_SDK%\Lib\vulkan-1.lib` at link time
3. In `local.rs`: `WhisperContextParameters { use_gpu: true, .. }` when building context
4. `vulkan-1.dll` ships with GPU drivers — no bundling needed

Best done as a separate PR after CPU-side fixes are verified.

## Suggested skills (next session)

- **`run`** or **`verify`** — launch the new binary and confirm < 5 s dictation
- **`superpowers:finishing-a-development-branch`** — once speed is confirmed, guides commit/PR/merge
- **`superpowers:systematic-debugging`** — if still slow, next hypothesis is OpenMP threading
- **`commit-push-pr`** — one-shot commit + PR when ready
