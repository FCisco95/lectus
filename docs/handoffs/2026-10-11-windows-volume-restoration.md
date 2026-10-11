# Windows volume restoration checkpoint

## Current follow-up: Windows volume restoration (2026-10-11)

User confirms that the **Windows master volume** sometimes remains at 1 or 4 after
Lectus mutes playback for dictation. Installed Windows binary now reports **0.7.1**;
Mac update/pairing is still unverified.

`src-tauri/src/playback.rs` now restores both the saved level and mute state, retains
its snapshot after failed writes for the next stop/idle/exit retry, and serializes
restore with new duck operations. Previously mute restoration only unmuted, and
`.take()` discarded the snapshot before successful OS writes. This reproduced
three regression failures (reduced level, failed volume restore, failed unmute).
All three pass after the patch. The original cause of the level dropping to 1/4
on the user's machine is **not yet established**; simulated level drops confirm
that Lectus previously left the reduced level unchanged. Windows endpoint changes
are not covered by this focused fix; the adapter still targets the default output.

Validation: 12 isolated playback tests passed; native suite 158 passed, 0 failed,
5 pre-existing ignored environmental/benchmark tests. One unrelated test,
`test_set_clipboard_text`, was excluded to avoid overwriting the user's clipboard.
Both real Windows endpoint tests passed. Existing compiler warnings remain.

The patch is not in the published 0.7.1 packages or installed application.
A local Windows release executable compiled successfully (56.71 s), without automatic deployment.
The staged executable is in the Input Leap workspace at
`setup-downloads/lectus-volume-fix/chirp.exe` (still version 0.7.1, local patch only).
SHA256: `E83CD527774730187FBAD1428DD14B16F18E1941A83CEEEFA7151B4A3353438C`. Ask for a safe
restart moment before replacing the running app: the user dictates through it.
Do not claim that the user's intermittent issue is resolved until a real dictation
preserves the selected Windows volume. No new release or version bump yet.

## Suggested skills

`systematic-debugging`, `verification-before-completion`, `handoff`.

## Generated artifacts this session

| What | Where it lives | Notes |
|---|---|---|
| Playback regression tests and correction | `src-tauri/src/playback.rs` | Not published or installed |
| Durable checkpoint | `docs/HANDOFF.md`, this file | Preserves the pending Mac setup |
| Local Windows build | Native Cargo target outside the repo | Build passed; staged copy in Input Leap workspace `setup-downloads/lectus-volume-fix/chirp.exe`; no credentials generated |

## Next-session prompt

```
Lectus 0.7.1 is published and installed on Windows; Mac pairing remains unverified.
The volume restoration fix has passed 158 native tests but is not installed.
Files: docs/HANDOFF.md, src-tauri/src/playback.rs, docs/REMOTE-DICTATION.md
Model: Codex Sonnet 5 (high) — focused implementation and verification, per user routing.
Skills: handoff-memory, systematic-debugging, verification-before-completion.
The local build passed. Arrange a safe Lectus restart, then verify that a
real dictation restores the Windows master volume. Do not claim the original
source of the volume drop is known. Resume Mac pairing after this issue.
```
