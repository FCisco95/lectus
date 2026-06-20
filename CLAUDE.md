# CLAUDE.md — Lectus

**Project:** Lectus — Organic-branded voice dictation app (Tauri 2 · Rust · React + TS). Hotkey → speak → Whisper transcribe (local `whisper-rs` or Groq cloud) → inject text into the focused field of any app. Repo renamed from `chirp` (internal crate may still read `chirp`/`chirp_lib`). Strategy/plans live in the cisco-brain vault: `docs/superpowers/plans/2026-06-01-chirp-voice-app-phase*.md`.

Keep this file short and execution-focused.

## Non-negotiables
- API keys live in env/config only — never paste into chat or commit.
- `tauri-plugin-global-shortcut`/muda CANNOT register a bare modifier (e.g. Right Ctrl) — it panics. Default hotkey is `Ctrl+Shift+Space`.
- Prefer small, focused diffs. Ask before changing the capture/injection or shortcut pipeline.

## Repo tooling
- **Agent-driven UI debug / visual verification** → `tauri-plugin-mcp` (Claude screenshots, inspects DOM, drives input on the running Tauri app — Playwright can't reach a Tauri webview). **NOT wired yet:** requires `npm i -g tauri-plugin-mcp-server` + adding the crate to `src-tauri/Cargo.toml` + registering it in the Tauri builder (dev builds only). Do in a dedicated Lectus session. Registry: cisco-brain `40 - RESOURCES/Claude Code Tooling — Install Registry.md`.
