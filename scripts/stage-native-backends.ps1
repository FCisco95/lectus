param([string]$ReleaseDirectory)
$ErrorActionPreference = 'Stop'
# A CMD/Visual Studio hop can preserve PowerShell 7's module search path when
# invoking Windows PowerShell. Load this host's built-in hashing module explicitly.
Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Utility/Microsoft.PowerShell.Utility.psd1') -ErrorAction Stop
if (!$ReleaseDirectory) { $ReleaseDirectory = Join-Path $PSScriptRoot '../src-tauri/target/release' }
$release = (Resolve-Path -LiteralPath $ReleaseDirectory).Path
$runtimeHash = (Get-FileHash -LiteralPath (Join-Path $release 'llama.dll')).Hash
$matches = @(Get-ChildItem -LiteralPath (Join-Path $release 'build') -Directory -Filter 'llama-cpp-sys-2-*' | Where-Object {
  $candidate = $_
  (Test-Path -LiteralPath (Join-Path $candidate.FullName 'out/backends/ggml-vulkan.dll')) -and @(Get-ChildItem -LiteralPath (Join-Path $candidate.FullName 'out') -Recurse -File -Filter 'llama.dll' | Where-Object {
    (Get-FileHash -LiteralPath $_.FullName).Hash -eq $runtimeHash
  }).Count -gt 0
})
if ($matches.Count -ne 1) { throw 'Expected exactly one backend build matching the release llama.dll.' }
$files = @(Get-ChildItem -LiteralPath (Join-Path $matches[0].FullName 'out/backends') -File -Filter '*.dll')
foreach ($relative in @('backends', 'deps/backends')) {
  $destination = [IO.Path]::GetFullPath((Join-Path $release $relative))
  if (!$destination.StartsWith($release + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Staging target escaped release output.' }
  [void][IO.Directory]::CreateDirectory($destination)
  foreach ($file in $files) { Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $destination $file.Name) -Force }
}
Write-Output "Staged $($files.Count) matched backend DLLs beside release and test executables."
