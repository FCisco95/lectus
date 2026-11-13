# NSIS acceptance in a disposable Windows guest

**Prepared, not executed.** The present `artifacts.json` deliberately cannot execute: the old NSIS executable/payload mapping is unverified, and no installer contains the current development build. Do not rebuild by bypassing `beforeBundleCommand`, mark legal documents approved, or substitute the old NSIS for the candidate.

## Host checks (read only)

```powershell
powershell -NoProfile -File scripts/diagnostics/installer-acceptance/run.ps1
pwsh -NoProfile -File scripts/diagnostics/installer-acceptance/run.ps1
powershell -NoProfile -File scripts/diagnostics/installer-acceptance/guard-checks.ps1
pwsh -NoProfile -File scripts/diagnostics/installer-acceptance/guard-checks.ps1
```

No installer, guest, directory or receipt is created by Preflight. `HypervisorPresent` is insufficient: WSL/VBS also sets it on physical PCs. The mutation guard requires a recognized VM model **and** a machine/snapshot attestation. WSL Ubuntu and Linux Docker do not run the Windows NSIS/WebView2/registry lifecycle.

`guard-checks.ps1` reads source and uses in-memory I/O stubs to check null/invalid hashes, missing files, receipt completeness/duplicates and PS5/PS7 JSON array normalization. It never calls mutating phases or launches an installer; passing these checks does not validate a guest lifecycle.

## Prepare the guest once an environment and eligible packages exist

1. Use a fresh disposable Windows VM, take a snapshot, install WebView2 through the guest's normal setup if missing, disconnect guest networking, and disable shared clipboard/profile/credential integration. Map/copy only this runner and verified public build artifacts; no host app-data folders, real settings, models, user history, secrets or accounts. Read-only artifact media plus guest-local receipts are preferable to a writable host share.
2. Review `artifacts.json` with exact NSIS hashes, expected embedded executable/uninstaller hashes and complete shipped runtime/resource lists (`installedFiles`: objects with relative `path` and `sha256`, e.g. `{"path":"chirp.exe","sha256":"<64 hex>"}`). Obtain these from verified packaging evidence, not by trusting a post-install snapshot. Keep the baseline and candidate distinct. This does not approve publication, legal identity or terms. Both package whitelists and files are required for every mutating phase; retain them throughout the guest experiment.
3. Inside the guest only, create `C:\LectusAcceptance\environment.json` with `disposable:true`, `computerName` equal to `$env:COMPUTERNAME`, `machineGuid` from `HKLM:\SOFTWARE\Microsoft\Cryptography`, and `snapshotReference` naming the actual clean snapshot. Retain this identity file in the guest, not in the portable report. Take the reviewed manifest SHA256 with `Get-FileHash`.
4. Set the variables below inside the guest. The explicit manifest hash is required for every mutating phase; each package is checked again immediately before execution. The script uses the guest user's normal `%LOCALAPPDATA%\Lectus` target, never an overridden environment variable pretending to isolate the host.

```powershell
$runner = 'C:\LectusAcceptance\kit\run.ps1'
$artifacts = 'C:\LectusAcceptance\artifacts'
$manifest = 'C:\LectusAcceptance\kit\artifacts.json'
$manifestHash = (Get-FileHash -LiteralPath $manifest -Algorithm SHA256).Hash
$common = @{ ArtifactDirectory=$artifacts; ManifestPath=$manifest; ExpectedManifestSHA256=$manifestHash }
& $runner @common -Phase Preflight
```

The runner never launches Lectus (`/R` absent), accepts app/model agreements, enables cloud, connects accounts, changes the publisher or enables autostart. No credential-manager claim is made: current inspected storage puts synthetic cloud/API credentials in `config.json`/`api.json`. App permissions, first-run UX and actual model loading need a separate interactive acceptance; sentinel model bytes are intentionally not loadable.

## Lifecycle branches

1. **Clean baseline and upgrade:** run `CleanInstall`, then `SeedData`, snapshot the seeded guest, then `Upgrade`. The runner verifies full whitelisted payload/uninstaller hashes and uninstall registration. SeedData and both upgrade phases require the verified baseline to be present; upgrade cannot silently become a clean install. Sentinels represent config, history, disabled cloud/API credentials and model bytes in Roaming and Local app-data roots; all eight exact entries must remain byte-identical. Empty, duplicate, incomplete or unrelated receipt paths are refused. It refuses to overwrite existing app data or receipts. Inspect start-menu/desktop shortcuts and the interactive setup manually without granting real agreements.
2. **Interrupted upgrade and recovery:** restore the seeded snapshot; run `InterruptUpgrade`. It attempts to terminate only its exact NSIS process after observing replacement of the main executable, recording whether this window was actually reached. Fast installers may finish first: a missed point is an inconclusive/failed case, never an interruption pass. Do not launch the interrupted app. Run `Repair` and verify all whitelisted files and sentinels. This covers process termination only; for power-loss/VM reset use a separate guest snapshot and external hypervisor control, capture the interruption point, and verify/reinstall before opening the app. The runner does not power off the host/guest.
3. **Uninstall retaining data:** from a verified candidate, run `UninstallPreserve`. NSIS `/S` leaves its remove-data checkbox unchecked; verify absent app executable, uninstall/Run registration, and retained sentinels. Inspect shortcut removal manually. `_?=` avoids the NSIS self-copy subprocess; the uninstaller executable itself can remain, so this is not a claim that the install directory is empty.
4. **Explicit data removal:** restore a candidate snapshot and use its interactive uninstaller with the remove-data checkbox deliberately checked. Leave the synthetic sentinels/receipt in place beforehand; do not simulate deletion with PowerShell. Run `VerifyDeletedData` afterward. This proves actual NSIS deletion of the sentinel files, not secure erasure, filesystem recovery resistance or deletion of external provider/account data.
5. **Collect evidence:** guest receipts are `C:\LectusAcceptance\receipts\<UTC>-<phase>.json`. Export these, reviewed artifact manifest, screenshots of explicit choices, final payload hashes, VM/snapshot description and exact versions. Keep failures and inconclusive interruption attempts. Do not export machine GUID, real credentials or host profiles. Destroy the disposable guest only after the evidence is reviewed.

Example phase invocation in the guest: `& $runner @common -Phase CleanInstall` (then repeat using the bounded phase names above). The kit intentionally does not create a VM, enable Windows features, download an OS image, change services, reboot, run NSIS on the host or generate a substitute installer.
