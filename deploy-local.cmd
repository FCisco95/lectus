@echo off
rem Dev-loop deploy: copy the freshly built release binaries (C:\lt\release, produced by
rem build-release.cmd / the Vulkan recipe) over the installed app in %LOCALAPPDATA%\Lectus,
rem so the desktop shortcut always runs the latest build. Handles a running instance by
rem killing it first (UAC-elevated fallback when chirp.exe runs elevated), then relaunches.

setlocal
set BUILD_DIR=C:\lt\release
set INSTALL_DIR=%LOCALAPPDATA%\Lectus
set EXE=chirp.exe

if not exist "%BUILD_DIR%\%EXE%" (
  echo ERROR: %BUILD_DIR%\%EXE% not found. Run build-release.cmd first.
  exit /b 1
)
if not exist "%INSTALL_DIR%" (
  echo ERROR: Install dir %INSTALL_DIR% not found. Install Lectus first.
  exit /b 1
)

rem 1) Kill a running instance (normal kill, then UAC-elevated taskkill as fallback).
for /f "tokens=2" %%p in ('tasklist /fi "imagename eq %EXE%" /fo list ^| findstr /b "PID:"') do (
  taskkill /PID %%p /F >nul 2>&1
)
for /f "tokens=2" %%p in ('tasklist /fi "imagename eq %EXE%" /fo list ^| findstr /b "PID:"') do (
  echo chirp.exe still running ^(PID %%p^) - killing with elevation...
  powershell -Command "Start-Process taskkill -ArgumentList '/PID %%p /F' -Verb RunAs -Wait" >nul 2>&1
)

rem 2) Copy binaries: exe, llama/ggml DLLs, ggml backends. Icons/models are untouched.
copy /y "%BUILD_DIR%\chirp.exe" "%INSTALL_DIR%\" >nul || goto copyfail
copy /y "%BUILD_DIR%\chirp_lib.dll" "%INSTALL_DIR%\" >nul || goto copyfail
copy /y "%BUILD_DIR%\ggml.dll" "%INSTALL_DIR%\" >nul || goto copyfail
copy /y "%BUILD_DIR%\ggml-base.dll" "%INSTALL_DIR%\" >nul || goto copyfail
copy /y "%BUILD_DIR%\llama.dll" "%INSTALL_DIR%\" >nul || goto copyfail
copy /y "%BUILD_DIR%\llama-common.dll" "%INSTALL_DIR%\" >nul || goto copyfail
for /d %%b in ("%BUILD_DIR%\build\llama-cpp-sys-2-*") do (
  if exist "%%b\out\backends" xcopy /y /q "%%b\out\backends\*.dll" "%INSTALL_DIR%\backends\" >nul
)

rem 3) Relaunch.
start "" "%INSTALL_DIR%\%EXE%"
echo Deployed %BUILD_DIR% -> %INSTALL_DIR% and relaunched.
exit /b 0

:copyfail
echo ERROR: copy failed — is chirp.exe still running?
exit /b 1
