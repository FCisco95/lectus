$ModelDir = Join-Path $PSScriptRoot "..\models"
New-Item -ItemType Directory -Force -Path $ModelDir | Out-Null
$ModelFile = Join-Path $ModelDir "ggml-tiny.en.bin"
if (Test-Path $ModelFile) {
    Write-Host "Model already exists at $ModelFile"
    exit 0
}
Write-Host "Downloading ggml-tiny.en.bin (~75MB)..."
Invoke-WebRequest -Uri "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin" `
    -OutFile $ModelFile
Write-Host "Done: $ModelFile"
