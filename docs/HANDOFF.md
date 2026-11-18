# Lectus - handoff

Updated: 2026-11-18. Branch: feat/remote-dictation. No tag, release or app replacement.
Previous handoff (access/Android Orca run): `docs/handoffs/archive/HANDOFF-2026-11-13-access-android-run.md`.

## Done 2026-11-18 (owner request: professional installer + section notices + signing)

- One-click installer: `src-tauri/windows/installer.nsi`, forked from the stock Tauri 2.11.2
  template (2d01631 is the unmodified copy, c1ac028 the changes). A double-click shows one
  borderless window ("Getting Lectus ready": logo, thin accent bar, light/dark, Windows
  language), installs in place and opens the app. The updater's `/P /UPDATE /R` path uses
  the same window. `/S` is silent and `/NOLAUNCH` skips the launch.
- One-click fixes over stock passive mode:
  - asks before closing a running Lectus;
  - never runs the old uninstaller, so shortcuts and the Run key survive;
  - asks before a downgrade;
  - adds the desktop shortcut only on a first install;
  - deletes stale `backends\*.dll`;
  - quiet WebView2 download;
  - failure state with a button.
- Config: language selector off (Terms are accepted in the app wizard), NSIS only on Windows,
  `@tauri-apps/cli` pinned 2.11.2. `scripts/check-nsis-template.mjs` (in
  beforeBundleCommand) fails if the stock template changes.
- Acceptance kit: `-InstallMode OneClick`, because `/S` skips the one-click logic.
- Signing hook (ba470dc): `src-tauri/tauri.signing.conf.json` plus `scripts/sign-windows.ps1`,
  merged only into release builds; it needs `LECTUS_SIGN_CERT_SHA1`. Tested with a temporary
  self-signed cert, which was removed. Tauri signs the exe, DLLs, backends, setup and
  uninstaller, and signs resource DLLs in place in the target dir.
- In-app discovery:
  - one spotlight per session, after the first dictation, for Vocabulary, Dictations,
    Models and AI cleanup;
  - sidebar "Get started" card: dictate once, add a word, start with Windows;
  - config `seen_spotlights` and `getting_started_hidden`;
  - files: `src/components/discovery/`, `src/styles/discovery.css`;
  - tests: `UI_SMOKE_DISCOVERY=1` and `scripts/discovery-smoke.mjs`.

## Verification

- Installer runs used side-by-side "Lectus Installer Test" builds: never published, and the
  legal gate ran without `--release`. Scenarios passed: fresh, same version, upgrade,
  downgrade (No), `/P /UPDATE`, `/S`, running-app prompt, locked-file failure, and both
  uninstalls. The real install hash, its Run key and its uninstall key were unchanged.
- Full payload: about 13 s in the window. The bar reaches 90% in about 5 s, then waits about
  7 s on the large model.
- App: `npm run build`. Discovery smoke 9/9, default 13/13, wizard 26/26.
  `test-release.cmd`: 312 passed, 0 failed, 13 ignored.
- Testing gotcha: from Git Bash set `MSYS_NO_PATHCONV=1`. Otherwise `/NOLAUNCH` turns into
  `C:/Program Files/Git/NOLAUNCH`, NSIS reads `/P` and runs passive.

## Not verified / known limits

- `/R` and the auto-launch at the end: launching the test copy could rewrite the real Run key
  (`autostart.rs` hard-codes `%LOCALAPPDATA%\Lectus\chirp.exe`).
- 150% and 200% DPI. The real-name title widths ("Getting Lectus ready" and its translations).
- Discovery in native WebView2 (real autostart plugin, hidden window).
- The failure button reads NSIS's localized "Cancel": NSIS resets the text and it can't be
  posted later.
- A real-name installer stays blocked by `legal/manifest.json`. Final acceptance means the
  owner runs it over his install, and that needs his OK at the time.

## Owner decisions

- 2026-11-18: no paid code-signing certificate for now (cost not justified before sales).
  Ship unsigned. The download page must show how to get past SmartScreen ("More info", then
  "Run anyway"). Updates through the in-app updater should not hit SmartScreen.
- Revisit if sales cover it. The cheapest option found is a Certum individual OV cloud
  certificate at about EUR 139-209 a year; Azure Artifact Signing excludes EU individuals.
  The hook (`tauri.signing.conf.json`) is ready.
- With a certificate: set `bundle.publisher`, which also fixes Apps & Features showing
  "organic", and keep reading the old `Software\organic\Lectus` key.
- The website "Open Lectus in 3 steps" page waits for a public site. The Microsoft Store /
  MSIX route is the only way to remove SmartScreen warnings entirely.

## Still open from the access/Android run (run_db09bc5a3181)

Seven follow-up assignments were submitted. Auth and policy returned `outcome_unknown`; follow
their Orca receipts and do not resend. The Android channel question and full design approval
are pending. Pointers: `docs/handoffs/2026-11-13-orca-architect-prompt.md`,
`docs/handoffs/2026-11-13-access-android-coordination.md`,
`docs/designs/2026-11-13-access-and-android.md`.

## Release gates (unchanged)

- `legal/manifest.json` is unapproved, and publisher, licence and identity are unresolved.
- No capture, injection or shortcut change, and no installed-app replacement, without the
  owner.
- Official ChatGPT/Claude login and real inference still need native acceptance. macOS is
  uncompiled.

## Next-session prompt

```text
Continue Lectus on feat/remote-dictation from docs/HANDOFF.md. The one-click installer, signing hook and in-app discovery are committed and tested on side-by-side test builds. Owner declined a paid signing certificate for now (ship unsigned). Next: a real-name candidate once legal approves, and owner-run acceptance (auto-launch, /R, DPI). Keep publication and installed-app gates closed.
```
