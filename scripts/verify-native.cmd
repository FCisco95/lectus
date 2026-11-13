@echo off
rem Compile and test without deploying or replacing the installed app.
setlocal
set VULKAN_SDK=C:\VulkanSDK\1.4.350.0
set PATH=%VULKAN_SDK%\Bin;%PATH%
set CMAKE_GENERATOR=Ninja
if not defined CARGO_TARGET_DIR set CARGO_TARGET_DIR=%~dp0..\src-tauri\target
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if errorlevel 1 exit /b %ERRORLEVEL%
cd /d "%~dp0..\src-tauri"
if "%~1"=="--translation-bench" (
  cargo test --release --lib bench_translation_local --no-run
  if errorlevel 1 exit /b 1
  powershell -NoProfile -File "%~dp0stage-native-backends.ps1" -ReleaseDirectory "%CARGO_TARGET_DIR%\release"
  if errorlevel 1 exit /b 1
  cargo test --release --lib bench_translation_local -- --ignored --nocapture
  if errorlevel 1 exit /b 1
  exit /b 0
)
if "%~1"=="--build" (
  cargo build --release
  if errorlevel 1 exit /b 1
  exit /b 0
)
if "%~1"=="--refresh-permissions" (
  cargo clean --release -p tauri -p tauri-plugin-shell -p tauri-plugin-single-instance -p tauri-plugin-global-shortcut -p tauri-plugin-os -p tauri-plugin-autostart -p tauri-plugin-updater -p tauri-plugin-process
  if errorlevel 1 exit /b 1
)
cargo test --release --lib -- --skip test_set_clipboard_text --skip windows_mute_roundtrip_restores
exit /b %ERRORLEVEL%
