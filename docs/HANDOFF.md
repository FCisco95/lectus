# Lectus - HANDOFF

## Metadata

- Last Updated: 2026-09-15 (Windows PC)
- Repository: `lectus` (github.com/FCisco95/lectus) — still private
- Branch: `master` (local persistence/autostart fixes uncommitted)
- Version in manifests: `0.5.0`
- Live Windows install: `%LocalAppData%\Lectus\chirp.exe` (local rebuild 2026-09-15, launched, Vulkan load + warmup ok)

## TL;DR

Audit of "previous version / model not saved / words not stored" found real
causes, not a broken Whisper model.

1. **Launch-at-login pointed at leftover 0.4.0** (`C:\lt\release\chirp.exe`, Aug 26).
   Desktop/Start Menu already targeted 0.5.0. After reboot, 0.4.0 started first;
   single-instance then kept that process when the 0.5.0 shortcut was clicked.
   **HKCU Run `Lectus` now points at `%LocalAppData%\Lectus\chirp.exe`.** New
   builds also rewrite leftover Run keys on launch.
2. **Settings auto-save clobbered `model_name`.** The hidden Settings webview
   kept a stale snapshot; changing any other setting wrote the old model back
   to `config.json`. `apply_ui_update` now preserves model + pill position;
   config writes are atomic.
3. **History was only saved after a successful inject.** Failed paste dropped
   the transcript. History now saves first.

Current config still has `ggml-large-v3-turbo.bin` + dictionary `Mycel, Claude`.
App relaunched; Vulkan 3080 load + 173 ms warmup confirmed.

## What to do next

1. Reboot once and confirm the tray pill is 0.5.0 (Settings → Models & About)
   and Large v3 Turbo is still Active. That is the real autostart proof.
2. Dictation: wait for the pill to go idle before starting the next phrase.
   Overlapping hold-to-talk while large-v3-turbo is still transcribing is
   silently skipped (`pipeline: skipped overlapping dictation`). Changing that
   needs a capture-pipeline change — ask first (AGENTS.md).
3. **Wispr Flow is disabled** (2026-09-15): Startup shortcut removed,
   `openAtLogin` set false, not running. App still installed if you want it
   later. Lectus login start is enabled (`%LocalAppData%\Lectus\chirp.exe`).
4. Updater is still inert (private repo 404). Organic token gate remains the
   gating item for public releases.

## PC inventory (2026-09-15)

| Copy | Path | Role |
|---|---|---|
| Installed 0.5.0 | `%LocalAppData%\Lectus\chirp.exe` | **live** (desktop, start menu, Run key) |
| Cargo target | `C:\lt\release\chirp.exe` | current local rebuild, not autostarted |
| Old NSIS | `C:\lt\release\bundle\nsis\Lectus_0.4.0_x64-setup.exe` | leftover installer, do not run |
| Config/history | `%APPDATA%\ai.organic.lectus\` | `config.json` + `history.json` (cap 100) |

## Code this session

- `src-tauri/src/autostart.rs` — prefer installed exe; rewrite leftover Run keys
- `src-tauri/src/config.rs` — `apply_ui_update`, atomic `save_to`
- `src-tauri/src/lib.rs` — save_config merge-then-write; history before inject; autostart repair; overlap log
- `src/components/Settings.tsx` — reload config on focus; patch `model_name` on `model-active`

Tests: `cargo test --lib` → 58 passed, 0 failed.

## Suggested skills

- `verify` — live dictation pass after the reboot check
- `handoff-memory` — this file
- `superpowers:brainstorming` — overlapping-dictation queue (only if user wants pipeline work)

## Next-session prompt

```text
Lectus persistence/autostart audit shipped 2026-09-15 (uncommitted on master).
Read docs/HANDOFF.md. Confirm after reboot: Run key still points at
%LocalAppData%\Lectus\chirp.exe, Settings shows v0.5.0, Large v3 Turbo still
Active. Then either commit these fixes or (only if asked) design a queue so a
second hold-to-talk is not dropped while large-v3-turbo is transcribing.
Skills: verify, handoff.
```
