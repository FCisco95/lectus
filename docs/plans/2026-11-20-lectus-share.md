# Lectus Share: keyboard, mouse and dictation across two computers

Design only, 2026-11-20. Nothing built. Follows `2026-11-18-input-leap-follow.md`, option 2
(own Rust engine inside Lectus). No Input Leap or Deskflow code may be copied (GPL); this design
is ours.

## Why now, and what the lag test said

- Owner, after a 10-minute session on the Mac through Input Leap: "almost never stuck, felt
  smooth". Network over the same run: no loss, median 11 ms, p95 95 ms. So lag is not the
  reason to build this. The reason is the product: one app, one click, dictation that follows
  the computer, instead of Input Leap plus Lectus.
- Consequence: v1 does not need UDP. One TCP connection with `TCP_NODELAY` is enough to match
  today's experience. Add a UDP pointer channel only if a measured run shows stalls.

## Owner direction (2026-11-20)

"Make it simpler: one click, almost a button, auto connect when in reach." So the link
connects by itself whenever the paired Mac is discovered and reconnects every second after a
drop; the only choice left is one toggle. Built into M1 already. M4 shrinks to first-time
pairing without copying a code.

## M1 status (2026-11-20)

Code: `src-tauri/src/share/` (link, pointer, windows, macos), toggle in the remote panel,
reuses the Send/Receive pairing, mDNS `_lectus-share._tcp`. Native tests 328 passed (16 new).
Not run on hardware yet; the Mac side is uncompiled until a `mac-check-*` run.

## v1 scope

- PC (Windows) shares its keyboard and mouse with one Mac. The Mac is to the left, as today.
- Mouse move, buttons, scroll, keyboard with the current modifier map (PC Ctrl = Mac Command).
- Dictation follows by construction: the dictation key pressed while the Mac is active sends
  the text to the Mac. No log reader, no guessing.
- Connect with one click from the paired Mac, found by mDNS. No address field.
- Out of v1: Mac controlling the PC, clipboard sync, more than two computers, file drag, the
  login screen and UAC prompts.

## How it works

**PC side (new module `share/`, Windows only).**
- A low-level mouse hook watches the cursor. When it touches the left edge of the virtual
  desktop for 50 ms, the Mac becomes active.
- While the Mac is active: the cursor is parked in the middle of the PC screen and hidden;
  mouse deltas come from Raw Input (they do not stop at the screen edge); the keyboard and
  mouse hooks swallow every event and forward it. The dictation key is the exception: it is
  kept for dictation and not forwarded.
- Back to the PC when the Mac reports that its cursor touched its right edge, when the link
  drops, or with a fixed escape key that always works (proposal: Scroll Lock, already a
  control key in today's Input Leap config).

**Mac side (new module, macOS only).**
- Receives events and posts them with `CGEventPost` (mouse moved and dragged, buttons, scroll,
  key down and up). Needs Accessibility, which Lectus already asks for.
- Windows scan codes are mapped to Mac key codes by a fixed table, with tests.
- Reports its own edge back to the PC.

**Link.**
- Reuses the pairing key and mDNS discovery that already exist for text
  (`remote_dictation/transport.rs`, `mod.rs`). A new message channel next to the one-shot text
  delivery: one long-lived TCP connection, encrypted frames, heartbeat every 1 s.
- Mouse moves are coalesced: at most one per 4 ms, latest wins. Keys and buttons are never
  dropped or reordered.
- Dictated text goes over the same connection, with the existing rules: history saved first,
  delivered at most once, never retried after submit.

**Safety (the costly part).**
- Stuck keys: when the link drops or the Mac stops being active, the Mac releases every key
  and button it pressed, and the PC forgets its held state.
- Watchdog: no heartbeat for 2 s means the PC takes the input back.
- Windows removes a hook that answers too slowly. The hooks only copy the event to a queue,
  and they are renewed like the dictation hook (`lib.rs:1590-1605`).
- Known limit: a non-elevated Lectus cannot see input while an elevated window on the PC has
  focus. Same as Input Leap today (not elevated).
- Input Leap and Lectus Share cannot both run. Share is off by default; the panel says to stop
  Input Leap first.

## Milestones

| | Deliverable | Gate |
|---|---|---|
| M0 | This branch compiles for macOS (CI runs only on `v*` tags, so a manual job or a build on the Mac) | app starts on the Mac |
| M1 | Mouse only: edge crossing, move, buttons, scroll, return, escape key, watchdog | 20 crossings each way, no lost cursor |
| M2 | Keyboard, modifier map, stuck-key release | type a paragraph on the Mac, kill the link mid-key 5 times, no stuck key |
| M3 | Dictation follows the active computer | 20 dictations alternating, 0 wrong computer, 0 duplicates |
| M4 | One-click Connect in the panel | link from zero without typing anything |
| M5 | 3 working days instead of Input Leap | owner keeps it on |

Each milestone runs as a dev build next to the installed 0.7.1. No installed-app replacement.
Rough size: PC 600-900 lines of Rust, Mac 400-600, panel about 100 of TS.

## Measured against Input Leap

Same 10-minute session and `ping` as 2026-11-19. Visible stutters, disconnects, and a 240 fps
phone video of key press to character on the Mac (10 samples) for both engines.

## Needs the owner

- **M1 changes the input capture pipeline** (new hooks that swallow input while the Mac is
  active). AGENTS.md requires asking first.
- A Mac build: either CI artifacts for the branch, or building on the Mac. The Accessibility
  grant on macOS is tied to the app signature, so an unsigned dev build may ask again after
  each rebuild.
