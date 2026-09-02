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

## Build & deploy (Windows)

```cmd
rem 1. Release build into C:\lt\release (vcvars + Vulkan SDK env)
build-release.cmd

rem 2. Deploy that build over the installed app and relaunch it
deploy-local.cmd
```

`deploy-local.cmd` kills any running `chirp.exe` (UAC-elevated kill as fallback — an
elevated instance refuses a normal `taskkill`), copies exe + DLLs from `C:\lt\release`
into `%LocalAppData%\Lectus`, and relaunches. Desktop shortcuts always target the
install dir, so after this they run the fresh build.

Production releases are tag-driven: push a `v*` tag → GitHub Actions builds Windows +
macOS, signs, and publishes a draft release with `latest.json` for the in-app updater.

```sh
node scripts/bump-version.mjs 0.5.1   # package.json + Cargo.toml + tauri.conf.json
git commit -am "chore: bump to 0.5.1"
git tag v0.5.1 && git push origin master v0.5.1
```

The workflow fails early if the tag and the three manifests disagree. Review the draft
release on GitHub, then publish it: installed apps pick it up on next launch (background
download, "Restart now" banner in Settings). Windows bundling of the llama/ggml DLLs +
`backends/` lives in `src-tauri/tauri.windows.conf.json`; macOS in `tauri.macos.conf.json`.

## Roadmap

- **Phase 2.5** — true hold-to-talk via a low-level keyboard hook; show the pill overlay during capture; editable hotkey capture.
- **Phase 3** — in-app model manager (download Whisper variants).
- **Phase 4** — code signing, auto-updater, onboarding, Organic token gate.
