@echo off
REM Copy the cargo release into the NSIS install so Start Menu, tray, and
REM login all launch the same binary the last build produced.
set "SRC=C:\lt\release"
set "DST=%LOCALAPPDATA%\Lectus"
if not exist "%DST%\chirp.exe" (
  echo No Lectus install at %DST% — skip deploy
  exit /b 0
)
if not exist "%SRC%\chirp.exe" (
  echo No build at %SRC%\chirp.exe
  exit /b 1
)
echo Deploying %SRC%\chirp.exe to %DST%
copy /Y "%SRC%\chirp.exe" "%DST%\chirp.exe" >nul
if errorlevel 1 (
  echo ERROR: %DST%\chirp.exe is in use. Stop Lectus by PID, then rerun.
  exit /b 1
)
if exist "%SRC%\chirp_lib.dll" copy /Y "%SRC%\chirp_lib.dll" "%DST%\chirp_lib.dll" >nul
if exist "%SRC%\ggml.dll" copy /Y "%SRC%\ggml.dll" "%DST%\ggml.dll" >nul
if exist "%SRC%\ggml-base.dll" copy /Y "%SRC%\ggml-base.dll" "%DST%\ggml-base.dll" >nul
if exist "%SRC%\llama.dll" copy /Y "%SRC%\llama.dll" "%DST%\llama.dll" >nul
if exist "%SRC%\llama-common.dll" copy /Y "%SRC%\llama-common.dll" "%DST%\llama-common.dll" >nul
set "ICONS=%~dp0src-tauri\icons"
if exist "%ICONS%\tray-idle.png" (
  if not exist "%DST%\icons" mkdir "%DST%\icons"
  copy /Y "%ICONS%\tray-idle.png" "%DST%\icons\" >nul
  copy /Y "%ICONS%\tray-recording.png" "%DST%\icons\" >nul
  copy /Y "%ICONS%\tray-transcribing.png" "%DST%\icons\" >nul
  copy /Y "%ICONS%\tray-template-idle.png" "%DST%\icons\" >nul
  copy /Y "%ICONS%\tray-template-recording.png" "%DST%\icons\" >nul
  copy /Y "%ICONS%\tray-template-transcribing.png" "%DST%\icons\" >nul
  if not exist "%SRC%\icons" mkdir "%SRC%\icons"
  copy /Y "%ICONS%\tray-idle.png" "%SRC%\icons\" >nul
  copy /Y "%ICONS%\tray-recording.png" "%SRC%\icons\" >nul
  copy /Y "%ICONS%\tray-transcribing.png" "%SRC%\icons\" >nul
)
echo Deployed to %DST%\chirp.exe
exit /b 0
