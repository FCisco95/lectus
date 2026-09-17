# Lectus — Home window (click-to-open)

- Date: 2026-09-17
- Status: Approved (approach 2 — Home absorbs History)
- Repo: `FCisco95/lectus`

## Problem

Clicking Lectus (desktop/Start shortcut, tray, pill) does not open a real app
window — only the tray icon is obvious. Settings scrolling feels wrong. History
is a buried tab. There is no “this week” picture of use.

## Approach (chosen: Home absorbs History)

One window. **Home** is the first tab and the dictation archive. The History tab
goes away. Login stays silent. Any user click opens Home.

Rejected: keep History as a seventh tab (nav overflow / nested scroll);
sparse dashboard + separate archive (more chrome).

## 1. When the window opens

- **Login / `--autostart`:** tray + pill only. No window.
- **Desktop / Start shortcut, second instance, tray left-click:** show the
  window on Home, focus it. If it was already visible, focus only (do not yank
  the user off Vocabulary).
- **Pill click:** open Home. Do **not** toggle recording. Drag still moves the
  pill. Hold-hotkey remains the only dictate gesture.
- **Tray right-click:** Home, Quit.
- **Close (X):** hide, do not quit (unchanged).
- Autostart Run key and the autostart plugin extra-args must be
  `chirp.exe --autostart`. Repair leftover keys that lack the flag.
- Onboarding still shows on first run. After it finishes, the window stays on
  Home.

## 2. Home tab

Stats, derived from `history.json` (no second store), rolling last 7 days:

- Words this week
- Dictations this week
- Stored total (honest about the cap)

Status line: active model, hotkey label, warming/ready.

Then the full dictation list (newest first): copy, optional filter, clear.
Empty state: “Hold Right Ctrl and talk — your words show up here.”

Chips/buttons jump to Vocabulary and Models.

Cap raised from 100 → 500 so “this week” is not empty after a busy day.
Subtitle: “Last 500 dictations on this device.”

## 3. Nav and scrolling

Tabs, in order: **Home, General, Dictation, Vocabulary, AI, Models**.

- `html` / `body` / `.settings-app` do not scroll.
- Only `.settings-content` scrolls vertically. The list is **not** a nested
  scrollport.
- Top bar does not create horizontal page scroll (wrap / shrink nav).
- Window title: `Lectus` (it is the app, not “Settings”).

## 4. Out of scope

Updater/installer wizard, Mycel aliases, app profiles, overlapping-hold queue,
focused-field context, voice commands, turning AI cleanup on.

## Testing

- `history::stats` / `word_count` unit tests (week window, empty, cap).
- Autostart command-line: desired Run value includes `--autostart`; leftover
  path without the flag is rewritten.
- Manual: login silent; clicking shortcut/tray/pill opens Home; scroll only in
  the content pane.
