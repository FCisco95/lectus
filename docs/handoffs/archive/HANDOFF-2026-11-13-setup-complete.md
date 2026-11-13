# Lectus - handoff

Updated: 2026-11-13. Branch: feat/remote-dictation. Tasks 1 to 11 complete in source.
No push, tag, app replacement or installation. Next: owner reviews the legal drafts.

## Implemented

- Two-pane setup wizard: language, unchanged legal gate, microphone, optional
  vocabulary, local shortcut practice, trial/membership and a one-time Home hint.
- Recommended Base/Small download and selection; latest requests wait for download
  and native activation. Fast tests offer Large v3 Turbo Compact.
- Local folder scan: top 30 suggestions, bounded files/bytes/depth/traversal,
  valid text only, no symlinks/reparse points. Only selected words are saved.
- Optional word checks save language-scoped sound-alikes only after confirmation.
- Earlier work includes the 7 day trial, sound-alikes and prompt token budget.
- Branded NSIS header/sidebar and uninstaller header use the existing master mark.

## Verification

Task 1 recorded 276 native passes; session start reported 297. Final Windows run:
- test-release.cmd: 310 passed, 0 failed, 13 ignored; C:/lt junction verified.
- npm run build: passed, 96 modules.
- Wizard: 25 functional scenarios; guided: 11; legal: 18. All passed.
- UI uses mocked IPC. Thirty wizard and sixteen guided captures covered light/dark
  layouts and small/default viewports; screenshots reviewed without overlap.
- Both installer BMPs are non-empty 24-bit files, 150x57 and 164x314. JSON/image keys
  match the installed Tauri schema. An actual installer was not built or accepted.

## Final review and authorized fixes

Independent source review of 3c97272..2cadb60 found three P2s, now covered by tests:
- Pending model choices are coalesced and wait for native load reservation release.
- Wizard capture stops/joins and preserves the final tail before inference.
- CPAL errors reach the wizard with a session tag; failed/partial audio is rejected.
  Stale errors are ignored and toggle practice restarts with the first new press.
The owner explicitly authorized the shared audio error observer. Default callers
retain log-only errors; pre-roll, resampling, injection and global hotkey behavior
were preserved. The misleading Dictionary voice-test hint was corrected.
Playwright cannot dispatch F13 here; both mode fixtures use ControlRight instead.
No physical microphone unplug, permission prompt or native close-window acceptance
was performed. macOS is uncompiled. Network-folder/parallel-scan load is unmeasured.
The pre-existing Unicode whole-word offset concern is unverified and separate.

## Owner inputs and release gates

- Approve the legal documents, publisher details and three draft sentences: trial,
  microphone use and folder scan. legal/manifest.json is unchanged/unapproved.
  Fresh-install and installer acceptance remain blocked by legal readiness.
- Record four owner WAV fixtures. Run the bias benchmark before changing prompt
  wording; no unmeasured prompt wording was shipped.
- English-sentence myself/Mycel ambiguity belongs to word learning. Next projects:
  English/Portuguese-only detection, Parakeet/Qwen3-ASR and learning from corrections.
- Older hardware/USB checks and real Windows-to-Mac insertion remain pending.
  Coordinate readiness before an observer or any running-app/candidate switch.

## Subscription AI still pending

Official ChatGPT/Claude login and completed real inference have not been accepted.
Historical candidate hashes and receipts are in the archived handoff; they predate
this wizard. Use a fresh current-branch build for any authorized native acceptance.
No accounts, provider permissions or installed files were changed by this work.

## History and artifacts

Implementation commits: 4854c62, b2b55f2, 94c38c4, d038417, 2cadb60; review fixes:
061a023. Prior work is b95e7f6..26f43a6, followed by 3c97272.
The old handoff and review-stop snapshot are in docs/handoffs/archive/.
Plan/spec and this plan's temporary execution workspace were removed after checks.
Canonical generated art is src-tauri/icons/installer-header.bmp and
src-tauri/icons/installer-sidebar.bmp; generator: scripts/gen_installer_art.ps1.
Smoke output, logs and temporary helpers from this work were removed. Existing
unrelated .context, docs/reviews and scripts/diagnostics work was preserved.
No keys, accounts, deployed resources or scheduled jobs were created. Npm audit
reported seven existing build-tool findings; dialog/API were not listed.

## Suggested skills

handoff-memory, verification-before-completion, handoff; verify and computer-use
only when native acceptance and human readiness are explicitly authorized.

## Next-session prompt

```text
Continue Lectus from docs/HANDOFF.md on feat/remote-dictation. The setup wizard, vocabulary and NSIS art are implemented and tested in source; nothing was installed or pushed. Native acceptance still awaits legal approval and readiness.
Files: docs/HANDOFF.md, legal/TERMS.md, legal/PRIVACY.md, legal/manifest.json, src/components/wizard/SetupWizard.tsx, scripts/ui-smoke.mjs.
Model: gpt-6.1-sol high (scoped follow-up and verification).
Skills: handoff-memory, verification-before-completion, handoff; verify/computer-use only for authorized live acceptance.
Start with the owner's final legal wording and publisher details. Keep publication and installation gates closed until separately authorized. Collect the four voice fixtures before deciding prompt wording. Preserve unrelated dirty work.
```
