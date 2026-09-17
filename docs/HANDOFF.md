# Lectus - HANDOFF

## Metadata

- Last Updated: 2026-09-17 late morning (Windows PC)
- Repository: `lectus` (github.com/FCisco95/lectus) — still private
- Branch: `master` (ahead of origin; Home-window work uncommitted)
- Version in manifests: `0.5.0`
- Live Windows install: `%LocalAppData%\Lectus\chirp.exe` (rebuilt 2026-09-17 10:09)

## TL;DR

Home window shipped locally. Clicking the tray, desktop/Start shortcut, or
the pill opens a Home tab (this-week word/dictation counts + full history).
Login stays silent (`Run` key is `chirp.exe --autostart`). History tab is gone.
Only the content pane scrolls.

Spec: `docs/superpowers/specs/2026-09-17-home-window-design.md`.

Still out of scope (user asked, not this slice): Mycel aliases / preset vocab,
Orca-style installer wizard, working auto-update (private repo 404), app
profiles, overlapping-hold queue.

## What to do next

1. User check: Home should have opened on this relaunch. Confirm stats + list
   scroll in one pane. Clicking the pill opens Home (hotkey still dictates).
2. Next slice if they want it: **dictionary that hears Mycel** (preset list +
   aliases), then **install + update**.
3. Push only if asked.

## PC inventory (2026-09-17)

| Copy | Path | Role |
|---|---|---|
| Installed + Home window | `%LocalAppData%\Lectus\chirp.exe` | **live** (10:09) |
| Cargo target | `C:\lt\release\chirp.exe` | same rebuild |
| Config/history | `%APPDATA%\ai.organic.lectus\` | `config.json` + `history.json` |

Autostart: `HKCU\...\Run\Lectus` = `...\Lectus\chirp.exe --autostart`.

## Constraints

- Do **not** change capture/injection/shortcut unless asked. Pill click no
  longer toggles recording (approved: opens Home).
- Default hotkey is still hold Right Ctrl.
- API keys stay in env/config only.

## Suggested skills

- `handoff-memory` — this file
- `verify` — optional; do not inject into user windows
- `superpowers:brainstorming` — Mycel dictionary / installer next

## Next-session prompt

```text
Lectus 2026-09-17: Home window shipped locally (click tray/shortcut/pill
opens Home; login silent with --autostart). Read docs/HANDOFF.md and
docs/superpowers/specs/2026-09-17-home-window-design.md. Next if they want
it: dictionary aliases for Mycel, then install+update. Do not reopen mute.
```
