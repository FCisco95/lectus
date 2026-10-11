# Remote dictation inside Lectus

User approved changing the capture/injection integration on 2026-10-11.

Goal: the existing Windows hotkey records the Alias; the paired Mac Lectus inserts the completed text. No microphone streaming, purchased receiver or clipboard race across Input Leap.

## Steps
- [x] Confirm Windows hotkey works while controlling Mac; normal clipboard transfer works but automatic paste uses stale text.
- [x] Implement opt-in paired receiver, encrypted text transport and mDNS discovery.
- [x] Integrate destination selection, receive-only capture suppression, delivery errors and history recovery.
- [x] Run frontend/native checks and transport integration tests (156 passed; explicit LAN discovery/delivery passed).
- [ ] Build signed Windows/macOS draft release and updater bridge.
- [ ] Pair both running builds and validate real insertion on the Mac (requires Mac access).

## Ownership
The primary agent owns all changed files: new remote_dictation modules, settings panel, lib.rs integration, Cargo dependencies, tests and documentation. No concurrent file writers.

## Contract and behavior
- Local is the default; persisted role is local, send or receive.
- Receiver generates a random pairing code; copy it once to the sender. Pairing secret stays in local app settings, never in discovery records or logs.
- mDNS advertises a random peer ID and ephemeral TCP port. Sender tracks the paired peer across DHCP changes. No IP text box in the normal flow.
- Text uses authenticated encryption (XChaCha20-Poly1305). Mutual challenge handshake, bounded frames/timeouts, no retransmission after submitting text, no automatic local paste on delivery failure.
- Receiver inserts via its existing native injection path, on the main queue where required on macOS; acknowledges only after insertion succeeds. A lost acknowledgement is reported as uncertain, not retried.
- Receive mode suppresses the local microphone/hotkey and checks existing license/accessibility prerequisites. Dictation history preserves failed outbound text for recovery.
- This is separate from the documented loopback transcription API, which remains unimplemented and loopback-only. The remote receiver accepts only paired final text, never audio or microphone commands.

## Verification
Protocol tests cover wrong secrets, tampering, replay, size limits, successful delivery, failed insertion and lost acknowledgement. Frontend build and repository Windows native tests/build are required. macOS insertion and mDNS across the actual two devices remain explicitly unverified until tested there.

## Release decision
The user chose to retain the current app-shell interface. The release therefore stays on
`feat/remote-dictation`, based on `be97b7f` (the existing app-shell/membership/release-routing
work). The alternative master-based adaptation is preserved locally in
`feat/remote-dictation-release` and is not the release source.

The Mac handoff on master mentions a local-only accessibility fix at `03588a1`;
that commit is unavailable in the remote branches. Re-check Accessibility and local-network
permission on the actual updated Mac. Do not claim end-to-end Mac insertion until it passes.

The LAN test caught Windows accepted sockets inheriting the nonblocking listener flag;
transport now explicitly restores blocking mode and bounds the total handshake/request time.
Remote paste now requires clipboard read-back confirmation before issuing the paste shortcut.
