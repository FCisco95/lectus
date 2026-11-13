# Package the existing release build without publishing or signing updater assets.
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
& node (Join-Path $repo 'scripts/check-legal.mjs') --release
if ($LASTEXITCODE -ne 0) { throw 'Installer distribution is blocked until legal publisher details and final documents are approved.' }
$release = Join-Path $repo 'src-tauri/target/release'
$runtimeHash = (Get-FileHash -LiteralPath (Join-Path $release 'llama.dll')).Hash
$candidates = @(Get-ChildItem -LiteralPath (Join-Path $release 'build') -Directory -Filter 'llama-cpp-sys-2-*' | Where-Object {
  $backend = Join-Path $_.FullName 'out/backends/ggml-vulkan.dll'
  (Test-Path -LiteralPath $backend) -and @(Get-ChildItem -LiteralPath (Join-Path $_.FullName 'out') -Recurse -File -Filter 'llama.dll' | Where-Object {
    (Get-FileHash -LiteralPath $_.FullName).Hash -eq $runtimeHash
  }).Count -gt 0
})
if ($candidates.Count -ne 1) { throw 'Cannot identify one backend build matching release/llama.dll. Rebuild or inspect the native artifacts before packaging.' }
$resourcePath = 'target/release/build/' + $candidates[0].Name + '/out/backends/*.dll'
$config = @{ bundle = @{ createUpdaterArtifacts = $false; resources = @{
  'target/release/build/llama-cpp-sys-2-*/out/backends/*.dll' = $null
} } }
$config.bundle.resources[$resourcePath] = 'backends/'
$configPath = Join-Path $release 'local-bundle.json'
$config | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $configPath -Encoding utf8
Push-Location $repo
try {
  & (Join-Path $repo 'node_modules/.bin/tauri.cmd') bundle --bundles nsis --config $configPath --no-sign --ci
  if ($LASTEXITCODE -ne 0) { throw 'Installer packaging failed.' }
} finally { Pop-Location }
