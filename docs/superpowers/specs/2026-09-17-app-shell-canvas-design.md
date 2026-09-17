# App shell — canvas layout and frameless window

**Date:** 2026-09-17
**Status:** approved, implementing
**Reference:** Wispr Flow (screenshots reviewed 2026-09-17)

## What this is

Lectus works, but it looks like a settings dialog wearing a Windows frame. Every
sidebar item is a settings tab, the Home page scrolls, and the native titlebar
does not belong to the app.

This spec covers the shell only: a frameless window with our own titlebar, a
sidebar of *surfaces* rather than settings, Settings moved into a modal, and a
Home page that fits on one screen.

Chosen structure: **A** (Settings becomes a modal). Chosen surface treatment:
**Canvas** — a bare sidebar sitting on the window background, content in one
rounded card with a margin.

## Not in this spec

Each gets its own spec later: the Insights page (WPM, per-app usage, streak
heatmap, milestone lines), the what's-new popup after an update, the mobile app
(a different product — no text injection on iOS/Android; it would be a keyboard
extension), and the model lineup review.

Untouched by definition: capture, injection, hotkeys, the transcription pipeline,
the Organic gate, and the pill window.

## The window

`tauri.conf.json`, `settings` window only:

| Key | From | To |
|---|---|---|
| `decorations` | `true` | `false` (Windows) |
| `width` / `height` | 880 / 640 | 960 / 680 |
| `minWidth` / `minHeight` | 720 / 540 | unchanged |

Windows 11 rounds undecorated top-level windows itself, so corner radius needs no
code.

**macOS differs deliberately.** `decorations: false` there deletes the traffic
lights, and rebuilding them is a bad trade. macOS keeps its native buttons via
`titleBarStyle: "Overlay"` with the content inset past them. `Titlebar.tsx`
renders our own buttons only under `html.platform-win`, a class `tokens.css`
already sets.

## Components

| File | Job |
|---|---|
| `Titlebar.tsx` | Drag region, logo mark + wordmark, minimise / maximise / close |
| `Shell.tsx` | Two-column grid: bare sidebar, content card. Owns the current surface |
| `Sidebar.tsx` | Home · Dictations · Vocabulary · Membership; bottom group Settings · Help · version |
| `SettingsModal.tsx` | Backdrop + panel with its own left rail; reuses the four config panels unchanged |
| `useConfig.ts` | Extracted from `Settings.tsx`: load, debounced save, "Saved" status |

`useConfig` is the one structural change. `Settings.tsx` currently owns both the
chrome and the config save/debounce logic; the shell and the modal both need
config, so it comes out as a hook. `Settings.tsx` becomes `Shell.tsx` and shrinks.

`App.tsx` renders `Titlebar` above whatever the window shows — including
onboarding. With `decorations: false`, a titlebar-less onboarding screen is a
window the user cannot move.

Window controls draw as Segoe Fluent Icons glyphs: `\uE921` minimise, `\uE922`
maximise, `\uE923` restore, `\uE8BB` close. Pixel-identical to Win11, no SVG to
maintain; the maximise glyph swaps on `onResized`.

Dragging: `data-tauri-drag-region` plus `app-region: drag`, with
`app-region: no-drag` on the buttons.

## Navigation

Sidebar: four surfaces, a divider, then Settings / Help / version at the bottom.
Active item is a soft fill, no left bar. `Icon.tsx` gains three glyphs: history,
gear, help.

Settings modal:

1. Opens from the sidebar gear or from a deep link.
2. Backdrop dims the shell. The panel is a rounded card with a left rail —
   General, Dictation, Models, AI cleanup — and version + "Check for updates" in
   its footer.
3. Closes on Esc, backdrop click, or ✕. Focus moves into the panel on open and
   returns to the gear on close.
4. Accepts an `initialTab`.

`initialTab` exists because two flows jump straight to a config page. After the
split they land in different places:

| Jump | Lands |
|---|---|
| Vocabulary, Membership | sidebar surface |
| Models | Settings modal, Models tab |
| `license-blocked` event | Membership surface, shell behind untouched |

Membership stays a surface, not a modal tab: a locked user should see the problem
and the fix on one screen.

## Home

Top to bottom: membership banner (only when it applies) → "Welcome back, <name>"
→ three stat cards → one status line → empty-state hint. Nothing else. At
960 × 680 it does not scroll, which is the point.

The three cards use data we already have: **words this week**, **dictations this
week**, **day streak**. Streak is derived from existing `history.json`
timestamps, so it works retroactively. WPM and per-app usage are *not* here —
they need new capture fields and belong to the Insights spec.

### The name

There is no account, so the default comes from the OS. `std::env::var("USERNAME")`
is free but returns the account name — `joao_`, not `João`. The real display name
needs `GetUserNameExW` from secur32, declared by hand the way `libc` already is
here, or a new `windows` crate dependency.

Config gains `display_name: String`; empty means derive. First run tries the OS
display name and falls back to a prettified username. Settings → General carries
the field at the top, so the worst case is typing a name once.

## Dictations

Today's list, filter, copy and clear, moved wholesale out of Home. It owns the
only scroll in the app. Scrollbars become thin and overlay-style, extending what
`settings.css:145` already does.

## Spike — before any layout work

Throwaway branch, ~30 minutes. Flip `decorations: false` and measure on Win11:

1. Resize borders — do the eight edges still grab?
2. Snap Layouts — hover-maximise flyout, and `Win` + arrow
3. Double-click titlebar to maximise; `Alt` + `Space` system menu
4. Multi-monitor DPI change while the window is open

Two decisions come out of it: whether we hand-roll six invisible 6 px edge strips
calling `startResizeDragging()`, and whether losing Snap Layouts is acceptable or
sends us back to a decoration plugin. **The findings go in the plan** — this is
exactly the kind of thing that gets re-learned three times.

## Verification

No test runner in `package.json`, and layout work does not justify adding one.

| Check | How |
|---|---|
| Types | `npm run build` (tsc) |
| Backend untouched | `cargo test --release --lib` — 115 tests, must stay 115 green |
| The window itself | live launch via the `verify` skill; `tauri-plugin-mcp` is unwired, so screenshots are hands-on |

Build with `build-release.cmd` (Ninja + vcvars + `CARGO_TARGET_DIR=C:\lt`). A
plain `cargo check` fails in the debug cmake path.

## Risks

- Drag region swallowing clicks on the greeting row.
- Focus trap in the modal (keyboard users, and Esc while a select is open).
- `settings.css`, `onboarding.css` and `tokens.css` churning at once.
- Dark theme parity — it needs its own pass, not a glance.

All of it is contained to the frontend plus one `tauri.conf.json` flag. Rollback
is that flag and a revert.
