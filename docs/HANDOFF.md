# Lectus — handoff

## Metadata
- Last Updated: 2026-10-24
- Repository: Lectus; local branch `main`; pre-existing source changes preserved.

## TL;DR
Mac installation updated; permission/restart loop repaired. Native microphone capture, local Whisper transcription and injection completion observed. Subscription provider login/inference remains unaccepted.

## Current Objective
Keep Mac dictation working; continue the separate subscription acceptance only when requested.

## Current State
- Installed `/Applications/Lectus.app`, version 0.7.1, freshly rebuilt for Apple Silicon from the working tree.
- Executable SHA256: `35f3b27203ef429e96f7ebbd2aad8b911001317b8df3b015e4c7fcf9f5e50f58`.
- Complete ad-hoc bundle signature; identifier `ai.organic.lectus`; deep/strict verification passes.
- Microphone and Accessibility grants match CDHash `6b30b93e56228aceec4aba1763cfe053d0c9c685`.
- Previous app retained at `/Applications/Lectus.app.before-20261024-microphone.backup`.
- Models/configuration/pairing preserved; history gained live dictations after installation.

## Recent Changes
- TCC explicitly rejected the old Accessibility grant: expected `9ff7589c98ac74d45b2865168cad38081b957e74`, running code `ff73c6fc7d939143a4c25c1e9bc173fb09d72bda`. Restart could not repair it. The old microphone grant already matched.
- Old bundle had a linker-only signature, unsealed resources and unbound Info.plist.
- Built into fresh `src-tauri/target/macos-20261024`; restored CMake/Tauri caches referenced the former project directory.
- Replaced executable plus matching native libraries/backends, signed the full bundle, renewed only Lectus permissions through macOS and relaunched. No capture/injection/shortcut source edits.

## Known Issues / Watch List
- Ad-hoc signing is not Developer ID/notarization; future rebuilt executables may need renewed grants. No public release made.
- Existing source/UI work remains dirty and uncommitted. Preserve it; do not include it casually in unrelated changes.
- Subscription browser login/inference still pending. Previous Windows receipts are historical, not Mac subscription acceptance.
- Windows USB/suspend, installer/legal and remote-delivery acceptance tasks remain in their existing handoffs.
- No remote `main` existed at session start. Local `main` began at `97d1ade`, matching the fetched remote-dictation head. Legacy branches preserved; no branch deletion/stash/worktree/rewrite.

## Quick Reference
- Portable repair evidence: `docs/reviews/2026-10-24-macos-permissions.md`.
- Private app copies, runtime logs, receipt and source hashes: `.context/macos-permission-2026-10-24/`.
- Prior subscription context: `docs/handoffs/2026-10-22-subscription-ai.md`, `docs/reviews/2026-10-22-subscription-ai-review.md`.
- A local scratch TextEdit document was opened for shortcut verification; no transcript is in the portable report.

## Validation
- Frontend: 77 modules; native release build: 2m39s, 40 existing warnings. No full unit-test rerun for this installation repair.
- `codesign --verify --deep --strict /Applications/Lectus.app` passes.
- Native app opened the MacBook Air microphone and loaded configured large-v3-turbo on Metal; warmup 1187 ms.
- Captures 1.21 s / 21.76 s; transcription 2588 / 2771 ms; injection completion 296 / 290 ms in native logs.
- Accessibility banner absent; no hook-install failure after final relaunch. Exact spoken-text accuracy/destination content not evaluated.
- Source baseline unchanged during build/install; configuration/history/pairing unchanged through installation, before live dictation updated history.

## Next Actions
1. Use the installed app normally. For any new failure inspect fresh logs and the installed signature before resetting permissions again.
2. When requested, complete official subscription logins and each card's Test inference; do not mark subscription features accepted beforehand.
3. Treat stable Developer ID signing as a separate release decision.

## Resume Checklist
- Verify running path/hash, current permissions and repo status; do not use old PIDs or Windows executable hashes as Mac evidence.
- Preserve existing source changes, app backups and provider/remote settings.

## Resume Prompt
Continue from the verified Mac repair. Consult the portable receipt and current runtime evidence before making changes; subscription acceptance remains separate.

## Suggested Skills
`systematic-debugging`, `computer-use`, `verification-before-completion`, `handoff-memory`. The `verify` skill currently targets Windows.
