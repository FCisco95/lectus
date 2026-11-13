@echo off
rem Real text translation diagnostic; no model download, capture, settings or installation.
setlocal
set VULKAN_SDK=C:\VulkanSDK\1.4.350.0
set PATH=%VULKAN_SDK%\Bin;%PATH%
set CMAKE_GENERATOR=Ninja
if not defined CARGO_TARGET_DIR set CARGO_TARGET_DIR=%~dp0..\src-tauri\target
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if errorlevel 1 exit /b 1
cd /d "%~dp0..\src-tauri"
cargo test --release --lib model_quality_test --no-run
if errorlevel 1 exit /b 1
powershell -NoProfile -File "%~dp0stage-native-backends.ps1" -ReleaseDirectory "%CARGO_TARGET_DIR%\release"
if errorlevel 1 exit /b 1
if not exist "%~dp0..\.context\translation-quality" mkdir "%~dp0..\.context\translation-quality"
set LECTUS_TRANSLATION_BENCH_MODEL=%~dp0..\models\Qwen3-4B-Q4_K_M.gguf
set LECTUS_TRANSLATION_BENCH_SHA256=7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5
set LECTUS_TRANSLATION_BENCH_OUTPUT=%~dp0..\.context\translation-quality\qwen-4b.json
call :measure
if errorlevel 1 exit /b 1
set LECTUS_TRANSLATION_BENCH_MODEL=%~dp0..\.context\text-models\Qwen3-0.6B-Q8_0.gguf
set LECTUS_TRANSLATION_BENCH_SHA256=9465e63a22add5354d9bb4b99e90117043c7124007664907259bd16d043bb031
set LECTUS_TRANSLATION_BENCH_OUTPUT=%~dp0..\.context\translation-quality\qwen-06b.json
call :measure
exit /b %ERRORLEVEL%
:measure
cargo test --release --lib model_quality_test::benchmark_translation_comparison -- --ignored --exact --nocapture
exit /b %ERRORLEVEL%
