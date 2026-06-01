# Lectus

> Voice dictation for any app. Hold a hotkey, speak, and your words appear in the focused text field — local or cloud.

**Lectus** (by Organic) is a Whisper-powered dictation app. The name nods to *lect-* (dialect, lecture — speech); the mascot is an Eclectus parrot. Built with Tauri (Rust) + React.

## Features

- **Tap-to-talk** dictation into any focused field (clipboard + paste injection).
- **Two backends, one toggle:**
  - **Local** — Whisper `ggml-tiny.en` runs fully offline, no account or key.
  - **Cloud** — Groq `whisper-large-v3-turbo` (~0.3s, higher accuracy) via an OpenAI-compatible API.
- **Settings UI** (from the tray menu) — switch backend, set the cloud API key/base URL.
- **System tray** with live state; floating pill overlay.
- Config persisted to disk; cross-platform (macOS + Windows).

## Quick start (dev)

```bash
# 1. Fetch the local model (~75MB)
bash scripts/download_model.sh   # or: pwsh scripts/download_model.ps1

# 2. Run the app
npm install
npm run tauri dev
```

Then: open a text field → tap **Ctrl+Shift+Space** → speak → pause. Open **Settings** from the tray icon to enable the Groq cloud backend.

## Tech

Tauri 2 · Rust (cpal, whisper-rs, reqwest, arboard, enigo, tauri-plugin-global-shortcut) · React 18 + TypeScript · Vite.

## Roadmap

- **Phase 2.5** — true hold-to-talk via a low-level keyboard hook; show the pill overlay during capture; editable hotkey capture.
- **Phase 3** — in-app model manager (download Whisper variants).
- **Phase 4** — code signing, auto-updater, onboarding, Organic token gate.
