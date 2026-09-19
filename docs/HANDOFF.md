## Mac transfer checkpoint — 2026-09-19

Resume branch: `feat/app-shell-canvas`. This is a preservation checkpoint, not a release or merge approval.

Checkpoint includes UI/branding/icons, shared history, settings focus fixes, ORGANIC-or-MYCEL eligibility and Windows autostart takeover changes after 86ff1ba. The older clean-757219b and ORGANIC-only descriptions below are historical. npm run build (TypeScript + Vite) passed on this tree. Native Rust/build and human visual checks were not rerun; verify Mac behavior before release.

Read the latest transfer report in cisco-brain `docs/orca/MAC-RESUME-2026-09-19.md` for the final commit and validation results. Current session evidence takes precedence over older clean/unpushed claims below.

### Suggested skills

`handoff-memory`; the repo's existing task-specific verification skills; `handoff` at session end.

### Next-session prompt

```text
Resume this repo on feat/app-shell-canvas. Read docs/HANDOFF.md, inspect Git status and compare HEAD with the Mac transfer report in cisco-brain. Preserve local Mac work. Continue only the documented next task after checking its remaining gates.
```

### Generated artifacts

This handoff overlay and docs/handoffs/2026-09-19-mac-transfer.md. No credentials or deployed resources generated.

---

# Lectus - HANDOFF

## Metadata

- Last Updated: 2026-09-17 evening (Windows PC, unattended agent run)
- Repository: `lectus` (github.com/FCisco95/lectus) — **public**
- Branch: `feat/app-shell-canvas` at `757219b`, **committed, NOT pushed**, tree clean
- `master` at `dfff2cb` (untouched). Version in manifests: `0.6.0`, released
- Live Windows install: `%LocalAppData%\Lectus\chirp.exe` (gate build, still 0.6.0)
- Fresh shell build: `C:\lt\release\chirp.exe` (from `757219b`, started at the end of the run)

## TL;DR

The **app-shell redesign is implemented** on `feat/app-shell-canvas`, per
`docs/superpowers/specs/2026-09-17-app-shell-canvas-design.md`, by an agent
running the brief in `docs/handoffs/2026-09-17-app-shell-agent-brief.md` with
nobody at the keyboard. Every commit passed the full gate (tsc + vite, release
build via `build-release.cmd`, `cargo test --release --lib`). Rust tests went
**115 → 134**, none dropped.

What nobody has done yet: **looked at it.** Screenshots need a human; see
"Needs a human" below before merging.

## What shipped (7 commits on top of the WIP `9514bb3`)

| Commit | What |
|---|---|
| `c5a44b9` | **Compiles.** `Shell` replaces `Settings`; new `DictationsPanel` (list, filter, copy, clear); Home is one screen; `Settings.tsx` + orphan `HistoryPanel.tsx` deleted; sidebar Help row opens the README via a fixed-URL command; `test-release.cmd` added |
| `7d8716a` | **Canvas CSS.** Bare sidebar, one glass content card with margin, modal styles, thin fading scrollbars, `--backdrop` token, dark parity via tokens only |
| `50eaa30` | **Greeting name.** `display_name` in `config.rs`; `identity.rs` calls `GetUserNameExW(NameDisplay)` from secur32 (hand-declared), falls back to a prettified `%USERNAME%`; field at the top of Settings → General |
| `8028458` | **Streak.** `streak_days` in `HistoryStats`, derived from `history.json` timestamps at local midnight; 8 tests |
| `eebc4ad` | **Resize strips** + the window permissions the titlebar needs (see below — this was a real bug in the WIP) |
| `06c8bab` | **Keyboard pass.** Shell goes `inert` under the modal, Tab wraps, Esc closes, focus returns to the gear, `aria-current` |
| `757219b` | **Extra 1: what's-new modal** shown once after an update (notes stashed in `whats_new.json` by "Restart now") |

Files that matter now: `src/components/{Shell,Sidebar,SettingsModal,Titlebar,ResizeHandles,WhatsNewModal}.tsx`,
`src/components/settings/{HomePanel,DictationsPanel,useConfig}.tsx`,
`src-tauri/src/{identity,history,updates}.rs`, `src-tauri/capabilities/settings.json`.

## Decisions the agent made alone (revisit if wrong)

1. **The WIP titlebar could not have worked.** `settings.json` capability had
   only `core:default`, which grants none of `start-dragging`, `minimize`,
   `toggle-maximize`, `close` or `start-resize-dragging`. All five are now
   listed explicitly. If dragging/buttons still misbehave, look here first.
2. **Only Home is scroll-free; the surface body scrolls for any surface that
   overflows** (Vocabulary with 64 words + 30 rules cannot fit 680 px).
   Dictations scrolls its own list so the filter box stays put. The spec's
   "Dictations owns the only scroll" was read as "Home must not scroll".
3. **Modal rail footer shows the version only.** "Check for updates" stays in
   Models → About (the panels were to be reused unchanged, and two buttons for
   one action is clutter). Spec wanted it in the footer — one-line change if
   the user disagrees.
4. **Home's status line: the model name is a link** that opens Settings →
   Models. That is the only path left from Home into the modal, satisfying the
   spec's jump table without the old Vocabulary/Models buttons.
5. **Greeting shows the first word of the OS display name** ("Welcome back,
   João"), the full string only when the user typed one in General.
6. **Streak timezone:** Rust std has no local time, so the webview passes
   `getTimezoneOffset()` to `get_history_stats`. No new crate.
7. **Help = README link**, via `open_help` (fixed URL, reuses
   `license::connect::open_in_browser`). Never lets the webview pick a URL.
8. **Modal close is a plain `×`**, not a Segoe glyph, so it renders on macOS.
9. **Insights (extra 2) parked.** It needs two new `HistoryEntry` capture
   fields written from the transcription worker — next to the do-not-touch
   pipeline, and not worth doing blind. Unstarted, nothing to undo.
10. Deleted `HistoryPanel.tsx` (already imported by nothing).

## Needs a human (unverified — nobody has seen the window)

- **Snap Layouts on the maximise button** (hover flyout, `Win`+arrow), native
  resize borders, double-click-to-maximise, `Alt`+`Space`, multi-monitor DPI.
  The spec's spike was never run; the six 6 px edge strips make resizing work
  regardless, and hide while maximised (`html.maximized`).
- Every surface and the modal in **both themes**: spacing, the glass card on
  the ambient wash, contrast of `--bg-active` as the sidebar's active fill.
- Drag region vs. clicks on the greeting row (spec risk #1).
- Onboarding under the titlebar (`.app-frame .onboarding-root` override).
- What's-new modal end to end: it needs a real update to fire. This machine
  has the installed 0.6.0 and a feed that also says 0.6.0, so it cannot be
  triggered until 0.6.1 is released.

## How to see it

```
powershell -Command "Get-Process chirp -ErrorAction SilentlyContinue | Select Id,Path"
# stop the C:\lt one by PID (never taskkill /IM), then:
powershell -Command "Start-Process 'C:\lt\release\chirp.exe'"
```
Window starts hidden — click the tray icon or the pill. Dark theme: Settings
(gear, bottom of sidebar) → General → Appearance.

## The gate (unchanged, now scripted)

```
npm run build
powershell -Command "& cmd.exe /c 'C:\Users\joao_\Desktop\DEVELOPMENTS\lectus\build-release.cmd'"
powershell -Command "& cmd.exe /c 'C:\Users\joao_\Desktop\DEVELOPMENTS\lectus\test-release.cmd'"
```
`test-release.cmd` is `build-release.cmd` with `cargo test --release --lib`.
134 tests. A running `C:\lt\release\chirp.exe` locks the exe and fails the
build with "Access is denied" — stop it by PID first. Plain `cargo check` is
still not a signal.

## Decisions made earlier (do not re-litigate)

1. **No subscription, no email accounts.** The wallet is the account.
2. **Not selling this as a product for now.** Holding ORGANIC stays the gate.
3. **Code signing is parked.** Azure Artifact Signing rejects individuals
   outside the US/Canada. Options if revisited: Certum individual OV
   (~€150/yr), an ENI to unlock Azure, or Apple Developer ($99/yr) for macOS.
4. **Shell structure A + Canvas** (this branch) — approved, implemented.

## What to do next

1. **Human pass on this branch** (list above), fix what looks wrong, then
   merge `feat/app-shell-canvas` → `master` and cut 0.7.0. `/code-review`
   before merging.
2. **Local transcription API** — spec exists at
   `docs/superpowers/specs/2026-09-17-local-transcription-api.md`, unstarted.
   `POST /v1/transcribe`, per-install token, origin allowlist, off by default,
   no remote mic control. Reuse the loopback server in `license/connect.rs`.
3. **Insights surface** — needs its own spec: speech duration + focused app on
   `HistoryEntry`, recorded going forward only; graphs start empty and say so.
4. Still unstarted: Mycel dictionary aliases, installer wizard polish, the
   three one-line gate decisions (skippable onboarding wallet step, no trial
   window for `Unlinked`, public-RPC rate limits).

## The gate, in one screen (Organic token gate — unchanged)

| | |
|---|---|
| Mint | `DuXugm4oTXrGDopgxgudyhboaf6uUg1GVbJ6jk6qbonk` (ORG, 6 decimals) |
| Floor | **$20** in dollars; token count follows the live price |
| Grace | 7 days from the last time the wallet was *seen above* the floor |
| Re-check | every 12 h, plus "Check again" in Membership |
| Storage | `license.json` in `%APPDATA%\ai.organic.lectus\` |

Code: `src-tauri/src/license/`; gate point `gate_blocks_dictation` in `lib.rs`.
Everything fails open inside grace. Not touched this session.

## PC inventory (2026-09-17 evening)

| Copy | Path | Role |
|---|---|---|
| Installed | `%LocalAppData%\Lectus\chirp.exe` | 0.6.0 gate build, autostart target |
| Cargo target | `C:\lt\release\chirp.exe` | **shell build from `757219b`** |
| Config/history/license | `%APPDATA%\ai.organic.lectus\` | `config.json` (gains `display_name`), `history.json`, `license.json`, `whats_new.json` (transient) |

## Constraints

- Do **not** change capture/injection/shortcut unless asked. Hotkey: hold Right Ctrl.
- API keys stay in env/config only. **No RPC key ships in the binary.**
- Prefer small, focused diffs. Heredocs for multi-line commits.
- Agent tooling note: the Bash tool here rejects long quoted heredocs
  ("unexpected EOF while looking for matching `'`") — write patch scripts /
  commit messages to the scratchpad with Write and run/`-F` them instead.

## Suggested skills

- `handoff-memory` — this file
- `verify` — drive the live app for the human pass; do not inject into user windows
- `code-review` — before merging the branch
- `superpowers:brainstorming` — Insights spec, Mycel aliases, installer wizard
- `superpowers:writing-plans` — the local transcription API, from its spec

## Next-session prompt

```text
Lectus 2026-09-17 evening: the app-shell redesign is implemented on
feat/app-shell-canvas (7 commits, unpushed, gate green, 134 tests) but no
human has seen the window. Read docs/HANDOFF.md first — "Needs a human"
lists what to look at (Snap Layouts, both themes, drag vs. clicks) and
"Decisions the agent made alone" lists what to revisit. Start the build at
C:\lt\release\chirp.exe, walk every surface and the modal, fix what is off,
then /code-review and merge to master. Do not touch capture, injection,
hotkeys, the gate or the pill.
```
