# Lectus local Windows installer — 2026-10-21

The user requested an installer to install and restart Lectus themselves. Packaging completed successfully; the installer has not been launched by the agent.

- Installer: src-tauri/target/release/bundle/nsis/Lectus_0.7.1_x64-setup.exe
- Size: 86,262,239 bytes.
- Installer SHA256: 9A8CA4CBCF17CDD6F95C024704DA6C8F5DD0803B894F0014F0E1ED7DDB5FCA5C
- Staged executable SHA256 after bundling: E35FE4B8865037CECC1DB06F372B9F3A53C9EDACE91EFFD741BC4FE0DB1068B1
- Still version 0.7.1, containing today's local platform/API changes. Same-version setup offers Add/reinstall; it is not a published release.
- Tauri temporarily patches the executable's bundle-type marker during packaging; the staged executable returned to its verified pre-bundle hash afterward. The embedded installer executable was not separately extracted/hash-checked.
- Packaging initially included two native backend caches. Rebuilt with only the set whose llama.dll matches the release runtime. Final NSIS manifest confirms 10 backend files and excludes the stale cache.
- scripts/package-local.ps1 reproduces matched-backend selection and local NSIS packaging without updater signing or publishing. Helper syntax validated; equivalent selection and bundler command executed successfully.
- Prior verification remains: 184 native tests; real local API Portuguese HTTP inference; 7 API UI scenarios and 8 captures. Native acceptance after installation is still pending.

## Next action

Quit Lectus from its tray menu, run the installer, choose Add/reinstall if prompted, and launch Lectus. Verify Local API appears in Settings and remains off by default. No user data or OS audio settings were changed by this packaging task.

## Suggested skills

handoff-memory, verify, computer-use, verification-before-completion, handoff.