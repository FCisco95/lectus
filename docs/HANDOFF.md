# Lectus - HANDOFF

## Metadata

- Last Updated: 2026-09-15 evening (Windows PC)
- Repository: `lectus` (github.com/FCisco95/lectus) — still private
- Branch: `master` (ahead of origin by 2 commits, not pushed)
- Version in manifests: `0.5.0`
- Live Windows install: `%LocalAppData%\Lectus\chirp.exe` (rebuilt 2026-09-15 evening)

## TL;DR

Mute-while-dictating shipped and the user confirmed it live: video kept
playing underneath, output stayed muted for the hold, then restored. That
is the intended Wispr Flow-style behavior.

Two local commits on master, not pushed:

- `afba49c` fix: persist selected model and repair leftover Windows autostart
- `e72d46b` feat: mute system playback while the dictation key is held

Session stopped for the night. Do not reopen mute design.

## What to do next

1. If building: **overlapping-hold queue** (recommended). With large-v3-turbo,
   a second hold while transcribing is still silently skipped
   (`pipeline: skipped overlapping dictation`). This is a capture-pipeline
   change — ask first (AGENTS.md). User was offered this vs context-awareness
   vs voice commands; no pick yet.
2. Reboot once and confirm tray is 0.5.0 + Large v3 Turbo still Active
   (autostart leftover-0.4.0 proof).
3. Open Settings once so `mute_while_dictating: true` is written to
   `config.json` (missing key already deserializes as on).
4. Push the two commits only if asked. Updater stays inert (private repo
   404) until an Organic token gate exists.

## PC inventory (2026-09-15)

| Copy | Path | Role |
|---|---|---|
| Installed 0.5.0 + mute | `%LocalAppData%\Lectus\chirp.exe` | **live** |
| Cargo target | `C:\lt\release\chirp.exe` | same rebuild |
| Config/history | `%APPDATA%\ai.organic.lectus\` | `config.json` + `history.json` |

Config: `ggml-large-v3-turbo.bin`, hold `RControl`, dictionary `Mycel, Claude`,
theme dark. `mute_while_dictating` may still be absent on disk.

## Constraints

- Do **not** change capture/injection/shortcut unless asked.
- Default hotkey is still hold Right Ctrl (low-level hook, not a bare-modifier
  plugin registration).
- API keys stay in env/config only.

## Suggested skills

- `handoff-memory` — this file
- `superpowers:brainstorming` — overlapping-dictation queue (only if user wants it)
- `verify` — optional; live mute is already user-confirmed
- `superpowers:systematic-debugging` — only if mute restore leaves the PC silent

## Next-session prompt

```text
Lectus mute-while-dictating shipped 2026-09-15 and the user confirmed it
live (video kept playing, stayed muted for the hold). Read docs/HANDOFF.md.
Two unpushed master commits: afba49c (persistence/autostart), e72d46b (mute).
Do not reopen mute. Next build candidate, if they want to keep going, is a
queue so a second hold is not dropped while large-v3-turbo is transcribing
(capture-pipeline — ask first). Otherwise: reboot autostart check, or
context-awareness / voice commands. Skills: handoff-memory, brainstorming.
```
