# Lectus — paired remote dictation checkpoint

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

## TL;DR
Remote dictation is implemented on `feat/remote-dictation`. Windows records/transcribes
with the existing microphone and hotkey; paired Mac Lectus receives encrypted final text
and pastes locally. User explicitly authorized the capture/injection integration and chose
to keep the current app-shell UI. No paid audio receiver or audio streaming is part of this solution.

Release `v0.7.1` is PUBLISHED in `FCisco95/lectus-releases` (user explicitly authorized publication on 2026-10-11). Both Windows and Mac packages and protocol tests
passed; both updater signatures were independently verified. The matching old-endpoint
bridge is also published. Both public latest/download/latest.json endpoints return 0.7.1;
Windows and Mac download URLs return HTTP 200. Next: guide in-app installation and pairing
on the two devices.
Actual Windows-to-Mac insertion remains unverified until both installed apps are updated.

## Source and release state

- Branch: `feat/remote-dictation`, based on the existing app-shell/membership/release-routing branch (`be97b7f`). The user chose to preserve that UI; do not switch back to the older master UI.
- Feature commits: `fd232eb`, `e373b87`; release preparation `c94833a`; CI ordering fix/version `f4627f2` (tag `v0.7.1`).
- CI: https://github.com/FCisco95/lectus/actions/runs/38134573650
- Published packages: https://github.com/FCisco95/lectus-releases/releases/tag/v0.7.1
- Published old-endpoint bridge: https://github.com/FCisco95/lectus/releases/tag/v0.7.1
- CI completed successfully for both platforms. The bridge feed was downloaded again and matched the verified main feed byte for byte.
- `v0.7.0` is a failed, unshipped tag (native tests initially ran before frontend generation). It was not rewritten. A master-based alternative is preserved locally as `feat/remote-dictation-release`; it is NOT the release source.
- Both releases were published in the required order after explicit user approval. No source visibility change, installed-app replacement, or repository merge has happened.

## Validation and limits

- Frontend production build passed. Windows native suite: 156 passed, 0 failed, 5 intentionally excluded environmental tests.
- The explicit LAN discovery/delivery test passed. It caught and fixed Windows accepted sockets inheriting nonblocking mode.
- Release CI: transport tests passed on Windows and macOS; both packages built and uploaded. Both updater signatures were independently verified. The Mac version, Apple Silicon architecture and local-network Info.plist entries were also checked.
- The installed Windows updater public key matches the release config. Its endpoint is still the old `FCisco95/lectus` repo; an identical `latest.json` bridge is published and publicly accessible there.
- Actual Mac paste, Accessibility/local-network prompts, UI appearance and in-app installation require the user's devices. The historical Mac handoff mentions local-only accessibility commit `03588a1`, which is absent from available remote branches. Check permissions on the installed update.

## Behavior and relevant files

See `docs/REMOTE-DICTATION.md` for the three-step setup and failure recovery;
`docs/plans/2026-10-11-remote-dictation.md` for the protocol/ownership contract.

Default destination is local. Send/Receive is explicit and persistent; it does not follow
the mouse automatically. Receiver mode disables its microphone. Same IPv4 LAN and multicast
access are needed; discovery tracks the paired receiver across IP changes. Text is encrypted,
mutually authenticated, bounded in size/time, and never retried after submission. Remote paste
requires matching clipboard read-back, so stale contents are not pasted on an unconfirmed write.

Windows history is written before delivery. An acknowledgement means the Mac completed its
paste command, not proof that a third-party text field consumed it. Keep that field focused.

The original Input Leap client/server IP configuration is a separate outstanding setup item.
The new discovery solves Lectus delivery only. A local hostname discovery probe returned no
response, so do not claim that using the Windows `.local` hostname fixes Input Leap yet.
VB-Cable on the Mac is unused by this route. Voicemeeter was downloaded but not installed.

## Next actions

1. Windows is confirmed on 0.7.1. Finish the local volume-fix build and arrange a safe restart/test. Verify Mac version via its update button; publication and public endpoint checks are already complete, so do not ask to republish 0.7.1.
2. Guide Mac Receive / Windows Send pairing, connection test, and one real dictated sentence. Keep source public.
3. Resolve the original Input Leap changing-IP setup separately; don't add paid audio apps.

## Suggested skills

`handoff-memory` to restore context; `systematic-debugging` if a device or CI check fails;
`verification-before-completion` before declaring the installed route working; `handoff`
when the session actually stops. All implementation files are owned by the primary agent.

## Generated artifacts this session

| What | Where it lives | Notes |
|---|---|---|
| Remote delivery implementation and tests | `src-tauri/src/remote_dictation/`, `src/components/settings/RemoteDictationPanel.tsx` | New settings file is created on each device only when used; no live pairing secret was generated for the user yet. |
| Setup and release notes | `docs/REMOTE-DICTATION.md`, `docs/release-notes-0.7.1.md` | Actual Mac installation/paste still pending. |
| Published releases and signed Windows/Mac packages | Linked GitHub releases above | Both published and verified; actual installation still pending. Existing GitHub signing secrets reused; no new credentials created or exposed. |
| Portable checkpoint | `docs/HANDOFF.md`, `docs/handoffs/2026-10-11-remote-dictation.md` | CI and bridge preparation complete. |

## Next-session prompt

```text
Lectus remote dictation is implemented on feat/remote-dictation, retaining the current UI
as the user requested. v0.7.1 packages and old-endpoint bridge are published; Windows
and Mac builds, protocol tests and updater signatures passed. No installed app has been replaced. Read docs/HANDOFF.md first.

Files: src-tauri/src/remote_dictation/mod.rs, src-tauri/src/remote_dictation/transport.rs, src/components/settings/RemoteDictationPanel.tsx, docs/REMOTE-DICTATION.md, .github/workflows/release.yml
Model: Codex Sonnet 5 (high), per the project routing convention — focused release completion and device debugging.
Skills: handoff-memory, systematic-debugging if needed, verification-before-completion, handoff at session end.

Publication is authorized and completed. Guide updating and pairing using the existing in-app update button.
Do not claim real Mac insertion until tested. Do not add paid audio apps or lose the existing UI.
```

## Verified updater artifacts

- Mac: `Lectus_aarch64.app.tar.gz`, SHA256 `fdeefd91a4a66b283e97e454a184ed188558e857dc0849f2f9b89ba29a724d77`.
- Windows: `Lectus_0.7.1_x64-setup.exe`, SHA256 `77a0defa815885f6c132054ec0d110f0fe7201c2abeb8e708bb6643294660915`.
- Signing keys were not read; validation used the embedded public key. The source tag is `f4627f2`; a later housekeeping commit aligns only the root package version in Cargo.lock plus this checkpoint.
