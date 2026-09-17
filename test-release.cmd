@echo off
set VULKAN_SDK=C:\VulkanSDK\1.4.350.0
set PATH=C:\VulkanSDK\1.4.350.0\Bin;%PATH%
set CMAKE_GENERATOR=Ninja
set CARGO_TARGET_DIR=C:\lt
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
cd /d C:\Users\joao_\Desktop\DEVELOPMENTS\lectus\src-tauri
cargo test --release --lib
exit /b %ERRORLEVEL%
