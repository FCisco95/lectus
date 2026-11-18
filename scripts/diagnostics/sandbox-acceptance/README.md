# Clean-PC acceptance in Windows Sandbox

Windows Sandbox is a disposable, clean Windows: no VC++ Redistributable and no GPU driver.
It shows what a new user's PC does with the installer, without touching this PC.

Needs the Windows Sandbox feature (`Containers-DisposableClientVM`, enabled 2026-11-18) and a
restart after enabling it.

```powershell
# 1. Side-by-side test installer from the current release build (never published).
pwsh -File scripts/diagnostics/sandbox-acceptance/build-test-installer.ps1 -Version 0.7.1
# 2. Run it in the sandbox. A sandbox window opens, runs by itself and closes.
pwsh -File scripts/diagnostics/sandbox-acceptance/run.ps1 -Installer $env:TEMP\lectus-test-installers\LectusInstallerTest_0.7.1.exe
```

`guest.ps1` runs inside the sandbox:

- **A.** No Vulkan loader: the one-click installer must show its graphics-driver message and
  install nothing.
- **B.** With a Vulkan loader copied in (still no GPU driver), the installer must install, keep
  the app-local MSVC runtime, open the app, and the app must stay up (CPU fallback). The app is
  then relaunched with `RUST_LOG=info` and its stderr saved.
- **C.** The silent uninstaller must remove the app.

Results land in `%TEMP%\lectus-sandbox-<time>\out`: `result.json`, screenshots
(`a-vulkan-message.png`, `b-install-*.png`, `b-after-launch.png`, `b-relaunch.png`) and
`app-stderr.log`. They are generated output: read them, never commit them. Use `-KeepOpen` to
keep the sandbox open after the run.
