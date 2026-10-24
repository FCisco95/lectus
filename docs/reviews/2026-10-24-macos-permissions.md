# Mac permission loop repair — 2026-10-24

The user requested an update because Lectus would not dictate and repeatedly requested a restart.

The installed app had an incomplete linker-only signature. The system TCC log explicitly reported an Accessibility code-requirement mismatch: the stored grant expected CDHash `9ff7589c98ac74d45b2865168cad38081b957e74`, while the current executable was `ff73c6fc7d939143a4c25c1e9bc173fb09d72bda`. Microphone access was already granted to that current executable. The UI showed Accessibility off despite System Settings showing a grant to the old code.

A fresh macOS arm64 release was built from the existing working tree, without source changes. Restored build caches contained absolute paths to the former project directory, affecting both CMake and generated Tauri permission metadata; a new Cargo target directory avoided reuse. Frontend build passed (77 modules), followed by a native release build in 2m39s (40 warnings). No full native unit-test suite was rerun for this installation repair.

The local candidate used the existing application bundle as its layout, replacing the executable, four native dylibs and four backend modules with this build's matching files and adding the current legal resources. Nested native files and then the complete application bundle were ad-hoc signed. The staged and installed applications both passed deep/strict codesign verification. This local update was not published, notarized or signed with a Developer ID certificate; the distribution/legal gates were not changed.

Installed executable SHA256: `35f3b27203ef429e96f7ebbd2aad8b911001317b8df3b015e4c7fcf9f5e50f58`.

Installed identifier: `ai.organic.lectus`. Designated requirement CDHash: `6b30b93e56228aceec4aba1763cfe053d0c9c685`.

The previous application is preserved at `/Applications/Lectus.app.before-20261024-microphone.backup`. Installation stopped only the Lectus process identified by its exact executable path. It did not replace configuration, history, models, license or pairing data. Configuration, history and pairing hashes remained unchanged through installation; subsequent live dictations naturally updated history.

Only this app's Microphone and Accessibility grants were reset. Native macOS authorization renewed both grants, and read-only TCC checks confirmed both requirements now match the installed code. After a clean relaunch, the Accessibility banner was absent and there was no keyboard-hook installation failure.

Native runtime evidence (UTC):

- 10:00:28: pre-roll opened `MacBook Air Microphone`.
- 10:00:33: configured local large-v3-turbo model warmed up on Metal in 1187 ms.
- 10:00:32–35: 1.21 s captured; local transcription completed in 2588 ms; injection reported completion in 296 ms.
- 10:00:40–10:01:05: 21.76 s captured; local transcription completed in 2771 ms; injection reported completion in 290 ms.

A separate synthetic Right Command press/release was attempted while a live dictation was transcribing; the overlap was safely skipped and is not counted as a successful independent dictation test. The evidence above comes from the actual installed native pipeline. Text insertion completion is based on native logs; exact spoken-text accuracy and destination content were not evaluated.

Private runtime logs, local app copies, source hashes and machine-readable receipt remain under `.context/macos-permission-2026-10-24/`. No transcript or credential is included in this portable report. Subscription account authentication/inference, cross-computer delivery and public-release acceptance were outside this repair.

Future locally rebuilt ad-hoc executables may require renewed permissions. This repair establishes a valid installation and matching grants for this exact executable; it does not establish stable signing across different builds.
