# Lectus — paired remote dictation checkpoint

## TL;DR
Remote dictation is implemented on `feat/remote-dictation`. Windows records/transcribes
with the existing microphone and hotkey; paired Mac Lectus receives encrypted final text
and pastes locally. User explicitly authorized the capture/injection integration and chose
to keep the current app-shell UI. No paid audio receiver or audio streaming is part of this solution.

Release `v0.7.1` is a DRAFT in `FCisco95/lectus-releases`. Both Windows and Mac packages and protocol tests
passed; both updater signatures were independently verified. The matching old-endpoint
bridge is also ready in draft. Next: obtain the publication decision, then guide installation
and pairing on the two devices.
Actual Windows-to-Mac insertion remains unverified until both installed apps are updated.

## Source and release state

- Branch: `feat/remote-dictation`, based on the existing app-shell/membership/release-routing branch (`be97b7f`). The user chose to preserve that UI; do not switch back to the older master UI.
- Feature commits: `fd232eb`, `e373b87`; release preparation `c94833a`; CI ordering fix/version `f4627f2` (tag `v0.7.1`).
- CI: https://github.com/FCisco95/lectus/actions/runs/38134573650
- Packages draft: https://github.com/FCisco95/lectus-releases/releases/tag/untagged-25f35f7fdd5a12977424
- Old-endpoint bridge draft: https://github.com/FCisco95/lectus/releases/tag/untagged-e33795e96508bc6cf872
- CI completed successfully for both platforms. The bridge feed was downloaded again and matched the verified main feed byte for byte.
- `v0.7.0` is a failed, unshipped tag (native tests initially ran before frontend generation). It was not rewritten. A master-based alternative is preserved locally as `feat/remote-dictation-release`; it is NOT the release source.
- No release publication, source visibility change, installed-app replacement, or repository merge has happened.

## Validation and limits

- Frontend production build passed. Windows native suite: 156 passed, 0 failed, 5 intentionally excluded environmental tests.
- The explicit LAN discovery/delivery test passed. It caught and fixed Windows accepted sockets inheriting nonblocking mode.
- Release CI: transport tests passed on Windows and macOS; both packages built and uploaded. Both updater signatures were independently verified. The Mac version, Apple Silicon architecture and local-network Info.plist entries were also checked.
- The installed Windows updater public key matches the release config. Its endpoint is still the old `FCisco95/lectus` repo; an identical `latest.json` bridge is now uploaded there as a draft.
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

1. Await the publication decision; both package and bridge drafts are ready.
2. When publication is authorized, publish `lectus-releases` first, then the old-endpoint bridge. Keep source public. Guide the user through updating both apps, Mac Receive / Windows Send pairing, connection test, and one real dictated sentence.
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
| Release drafts and signed Windows/Mac packages | Linked GitHub drafts above | Both unpublished and verified. Existing GitHub signing secrets reused; no new credentials created or exposed. |
| Portable checkpoint | `docs/HANDOFF.md`, `docs/handoffs/2026-10-11-remote-dictation.md` | CI and bridge preparation complete. |

## Next-session prompt

```text
Lectus remote dictation is implemented on feat/remote-dictation, retaining the current UI
as the user requested. v0.7.1 packages and old-endpoint bridge are ready in draft; Windows
and Mac builds, protocol tests and updater signatures passed. No installed app has been replaced. Read docs/HANDOFF.md first.

Files: src-tauri/src/remote_dictation/mod.rs, src-tauri/src/remote_dictation/transport.rs, src/components/settings/RemoteDictationPanel.tsx, docs/REMOTE-DICTATION.md, .github/workflows/release.yml
Model: Codex Sonnet 5 (high), per the project routing convention — focused release completion and device debugging.
Skills: handoff-memory, systematic-debugging if needed, verification-before-completion, handoff at session end.

After the publication decision, publish packages first and bridge second, then guide updating and pairing.
Do not claim real Mac insertion until tested. Do not add paid audio apps or lose the existing UI.
```

## Verified updater artifacts

- Mac: `Lectus_aarch64.app.tar.gz`, SHA256 `fdeefd91a4a66b283e97e454a184ed188558e857dc0849f2f9b89ba29a724d77`.
- Windows: `Lectus_0.7.1_x64-setup.exe`, SHA256 `77a0defa815885f6c132054ec0d110f0fe7201c2abeb8e708bb6643294660915`.
- Signing keys were not read; validation used the embedded public key. The source tag is `f4627f2`; a later housekeeping commit aligns only the root package version in Cargo.lock plus this checkpoint.
