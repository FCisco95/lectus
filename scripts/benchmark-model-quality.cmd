@echo off
rem Real model-only stress; never downloads, records, installs or changes settings.
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
cargo test --release --lib model_quality_test -- --skip benchmark_model_quality
if errorlevel 1 exit /b 1
if not exist "%~dp0..\.context\quality-stress" mkdir "%~dp0..\.context\quality-stress"
for %%M in (ggml-base.bin ggml-large-v3-turbo.bin) do (
  set LECTUS_BENCH_MODEL=%%M
  set LECTUS_BENCH_OUTPUT=%~dp0..\.context\quality-stress\%%M.json
  call :measure
  if errorlevel 1 exit /b 1
)
exit /b 0
:measure
cargo test --release --lib model_quality_test::benchmark_model_quality -- --ignored --exact --nocapture
exit /b %ERRORLEVEL%
