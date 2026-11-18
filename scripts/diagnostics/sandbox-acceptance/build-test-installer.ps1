# Side-by-side installer test build ("Lectus Installer Test"), never published.
# Different productName, identifier and main binary, so it never touches the installed Lectus
# or its running chirp.exe. The legal gate runs without --release, as the approved test plan
# allows. Needs an existing release build (cargo build --release with the vcvars env).
# Never run the test app on the host: autostart.rs and config.rs use the real Lectus paths.
param(
  [string]$Version = '0.7.1',
  [switch]$Fast,            # leave out the speech model and third-party notices (smaller)
  [switch]$NoPublisher,     # build like versions before bundle.publisher (manufacturer "organic")
  [string]$Template = '',   # alternative NSIS template, for test variants only
  [string]$Tag = '',
  [string]$OutDir = (Join-Path $env:TEMP 'lectus-test-installers')
)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\..'))
$release = Join-Path $repo 'src-tauri\target\release'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# Same backend selection as scripts/package-local.ps1.
$runtimeHash = (Get-FileHash -LiteralPath (Join-Path $release 'llama.dll')).Hash
$candidates = @(Get-ChildItem -LiteralPath (Join-Path $release 'build') -Directory -Filter 'llama-cpp-sys-2-*' | Where-Object {
  $backend = Join-Path $_.FullName 'out/backends/ggml-vulkan.dll'
  (Test-Path -LiteralPath $backend) -and @(Get-ChildItem -LiteralPath (Join-Path $_.FullName 'out') -Recurse -File -Filter 'llama.dll' | Where-Object {
    (Get-FileHash -LiteralPath $_.FullName).Hash -eq $runtimeHash
  }).Count -gt 0
})
if ($candidates.Count -ne 1) { throw "Expected one matching backend build, found $($candidates.Count)." }

$resources = [ordered]@{ 'target/release/build/llama-cpp-sys-2-*/out/backends/*.dll' = $null }
$resources['target/release/build/' + $candidates[0].Name + '/out/backends/*.dll'] = 'backends/'
if ($Fast) {
  $resources['../models/ggml-tiny.en.bin'] = $null
  $resources['../legal/third-party/'] = $null
}
$bundle = [ordered]@{ createUpdaterArtifacts = $false; resources = $resources }
if ($NoPublisher) { $bundle['publisher'] = $null }
if ($Template) { $bundle['windows'] = @{ nsis = @{ template = $Template } } }
$config = [ordered]@{
  productName = 'Lectus Installer Test'
  mainBinaryName = 'lectus-installer-test'
  identifier = 'ai.organic.lectus.installertest'
  version = $Version
  build = @{ beforeBundleCommand = 'node scripts/check-legal.mjs && node scripts/check-nsis-template.mjs && node scripts/check-windows-bundle.mjs' }
  bundle = $bundle
}
$configPath = Join-Path $OutDir "test-bundle-$Version$Tag.json"
$config | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $configPath -Encoding utf8

$testExe = Join-Path $release 'lectus-installer-test.exe'
Copy-Item -LiteralPath (Join-Path $release 'chirp.exe') -Destination $testExe -Force
Push-Location $repo
try {
  & node node_modules/@tauri-apps/cli/tauri.js bundle --bundles nsis --config $configPath --ci --no-sign
  if ($LASTEXITCODE -ne 0) { throw "tauri bundle failed ($LASTEXITCODE)" }
} finally {
  Pop-Location
  Remove-Item -LiteralPath $testExe -Force -ErrorAction SilentlyContinue
}

$setup = Get-ChildItem -LiteralPath (Join-Path $release 'bundle\nsis') -Filter "Lectus Installer Test_${Version}_x64-setup.exe" | Select-Object -First 1
if (-not $setup) { throw 'setup.exe not produced' }
$suffix = if ($Fast) { '-fast' } else { '' }
$dest = Join-Path $OutDir "LectusInstallerTest_$Version$suffix$Tag.exe"
Move-Item -LiteralPath $setup.FullName -Destination $dest -Force
Write-Output "built $dest ($([math]::Round((Get-Item $dest).Length/1MB,1)) MB)"
