# Clean-PC acceptance for a Windows installer in Windows Sandbox (host side).
# Needs the Windows Sandbox feature (Containers-DisposableClientVM) and a restart after enabling it.
# The sandbox is disposable: nothing it does reaches this PC except C:\LectusOut, which maps to
# the work folder printed at the end. Audio, video, clipboard and printers are not shared.
param(
  [Parameter(Mandatory)] [string]$Installer,
  # A Vulkan loader for scenario B; the sandbox has no GPU driver. Defaults to this PC's copy.
  [string]$VulkanLoader = (Join-Path $env:SystemRoot 'System32\vulkan-1.dll'),
  [int]$TimeoutSec = 1200,
  [switch]$KeepOpen
)
$ErrorActionPreference = 'Stop'
$sandbox = Join-Path $env:SystemRoot 'System32\WindowsSandbox.exe'
if (-not (Test-Path $sandbox)) { throw 'Windows Sandbox is not available. Enable Containers-DisposableClientVM and restart.' }
if (Get-Process -Name 'WindowsSandbox*' -ErrorAction SilentlyContinue) { throw 'A Windows Sandbox is already open. Close it first (only one can run).' }
$Installer = (Resolve-Path -LiteralPath $Installer).Path

$work = Join-Path $env:TEMP ('lectus-sandbox-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
$inDir = Join-Path $work 'in'
$outDir = Join-Path $work 'out'
New-Item -ItemType Directory -Force -Path $inDir, $outDir | Out-Null
Copy-Item -LiteralPath $Installer -Destination (Join-Path $inDir 'setup.exe')
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'guest.ps1') -Destination $inDir
if ($VulkanLoader -and (Test-Path -LiteralPath $VulkanLoader)) { Copy-Item -LiteralPath $VulkanLoader -Destination (Join-Path $inDir 'vulkan-1.dll') }

$guestArgs = if ($KeepOpen) { ' -KeepOpen' } else { '' }
$wsb = @"
<Configuration>
  <vGPU>Disable</vGPU>
  <Networking>Enable</Networking>
  <AudioInput>Disable</AudioInput>
  <VideoInput>Disable</VideoInput>
  <ClipboardRedirection>Disable</ClipboardRedirection>
  <PrinterRedirection>Disable</PrinterRedirection>
  <MappedFolders>
    <MappedFolder>
      <HostFolder>$inDir</HostFolder>
      <SandboxFolder>C:\LectusIn</SandboxFolder>
      <ReadOnly>true</ReadOnly>
    </MappedFolder>
    <MappedFolder>
      <HostFolder>$outDir</HostFolder>
      <SandboxFolder>C:\LectusOut</SandboxFolder>
      <ReadOnly>false</ReadOnly>
    </MappedFolder>
  </MappedFolders>
  <LogonCommand>
    <Command>powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\LectusIn\guest.ps1$guestArgs</Command>
  </LogonCommand>
</Configuration>
"@
$wsbPath = Join-Path $work 'lectus.wsb'
Set-Content -LiteralPath $wsbPath -Value $wsb -Encoding UTF8
Start-Process -FilePath $sandbox -ArgumentList "`"$wsbPath`""
Write-Output "sandbox started; work folder $work"

$done = Join-Path $outDir 'done.json'
$deadline = (Get-Date).AddSeconds($TimeoutSec)
while (-not (Test-Path $done) -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 5 }
if (-not (Test-Path $done)) { Write-Output "TIMEOUT after $TimeoutSec s; partial results in $outDir" }
$resultPath = Join-Path $outDir 'result.json'
if (Test-Path $resultPath) { Get-Content -LiteralPath $resultPath -Raw }
Write-Output "results: $outDir"
