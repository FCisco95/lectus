@echo off
rem Record a 6 second owner fixture for the vocabulary bias benchmark.
rem Usage: scripts\record-fixture.cmd pt_mycel_1
rem Name prefix pt_ or en_ selects the language. Output is git-ignored:
rem src-tauri\tests\audio_samples\owner\<name>.wav
if "%~1"=="" (
  echo Usage: scripts\record-fixture.cmd NAME   e.g. pt_mycel_1
  exit /b 2
)
set LECTUS_FIXTURE_NAME=%~1
set VULKAN_SDK=C:\VulkanSDK\1.4.350.0
set PATH=C:\VulkanSDK\1.4.350.0\Bin;%PATH%
set CMAKE_GENERATOR=Ninja
set CARGO_TARGET_DIR=C:\lt
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
cd /d %~dp0..\src-tauri
cargo test --release --lib record_owner_fixture -- --ignored --nocapture
exit /b %ERRORLEVEL%
