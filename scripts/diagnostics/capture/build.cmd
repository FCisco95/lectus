@echo off
setlocal
rem Standalone diagnostic only. Never builds or installs the Tauri app.
set CARGO_TARGET_DIR=%~dp0target
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if errorlevel 1 exit /b %ERRORLEVEL%
cargo build --offline --manifest-path "%~dp0Cargo.toml"
exit /b %ERRORLEVEL%
