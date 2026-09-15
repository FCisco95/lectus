# Lectus - HANDOFF

## Metadata

- Last Updated: 2026-09-15 (Windows PC)
- Repository: `lectus` (github.com/FCisco95/lectus) — still private
- Branch: `master` (ahead of origin by persistence commit + mute feature)
- Version in manifests: `0.5.0`
- Live Windows install: `%LocalAppData%\Lectus\chirp.exe` (rebuilt 2026-09-15 evening, launched)

## TL;DR

Mute-while-dictating shipped. Hold the key → default render endpoint mutes
(or volume-ducks to 0 if mute is unsupported) → restore on release, pipeline
error, idle, and process exit. Settings toggle **Mute playback while dictating**
defaults ON. Old `config.json` without the key loads as on.

Wispr Flow prior art: Settings → System → Sound → “Mute music while dictating”
(on by default on Windows; mutes the default output device, restores after).
Lectus matches that on Windows via `IAudioEndpointVolume`, not capture/VAD/inject.

Persistence/autostart audit from earlier today is committed
(`fix: persist selected model and repair leftover Windows autostart`).

## What to do next

1. **Try a real hold with music/YouTube playing.** Live synthetic dictation
   verify did **not** finish: the desktop was in use (Chrome focused) and the
   inject window kept disappearing. Evidence that *does* exist:
   - `cargo test --lib` → 68 passed, including restore-on-every-exit-path
     and a real `IAudioEndpointVolume` mute roundtrip.
   - New binary Vulkan-loaded large-v3-turbo (warmup 202 ms) before the
     synthetic pass was aborted.
2. Reboot once and confirm tray is 0.5.0 + Large v3 Turbo still Active
   (autostart proof from the morning audit).
3. Updater is still inert (private repo 404). Organic token gate remains
   the gating item for public releases.

## PC inventory (2026-09-15)

| Copy | Path | Role |
|---|---|---|
| Installed 0.5.0 + mute | `%LocalAppData%\Lectus\chirp.exe` | **live** (desktop, start menu, Run key) |
| Cargo target | `C:\lt\release\chirp.exe` | same rebuild |
| Config/history | `%APPDATA%\ai.organic.lectus\` | `config.json` + `history.json` (cap 100) |

Current config: `ggml-large-v3-turbo.bin`, hold `RControl`, dictionary
`Mycel, Claude`, theme dark. `mute_while_dictating` is not yet written to
disk; missing key deserializes to `true`.

## Code this session

- `src-tauri/src/playback.rs` — ducker + Fake tests + Windows endpoint volume
- `src-tauri/src/lib.rs` — hold-start duck; restore on hold-stop / idle /
  pipeline error / `RunEvent::Exit`
- `src-tauri/src/config.rs` — `mute_while_dictating` default true, atomic save unchanged
- `src/components/settings/GeneralPanel.tsx` — toggle next to VAD
- `src-tauri/Cargo.toml` — `Win32_Media_Audio` + Endpoints + Com features

Do **not** change capture/injection/shortcut unless asked. Default hotkey
is still hold Right Ctrl (low-level hook, not a bare-modifier plugin registration).

Tests: `cargo test --lib` → 68 passed, 0 failed, 4 ignored.

## Suggested skills

- `verify` — one live dictation pass with music playing (read config hotkey
  first; do not use Notepad as the inject target)
- `handoff-memory` — this file
- `superpowers:systematic-debugging` — if mute restore leaves the PC silent

## Next-session prompt

```text
Lectus mute-while-dictating shipped 2026-09-15 on master. Read docs/HANDOFF.md.
Confirm: hold Right Ctrl with YouTube/music playing — output goes quiet, then
restores on release. Settings → General shows “Mute playback while dictating”
on. config.json should pick up mute_while_dictating: true after any Settings
save. Live synthetic verify was aborted (user at desktop); re-run the verify
skill if you want log evidence. Capture/injection/shortcut pipeline stays off
limits unless asked.
Skills: verify, handoff.
```
