# Runs inside Windows Sandbox (Windows PowerShell 5.1, started by the .wsb LogonCommand).
# A clean Windows: no VC++ Redistributable, no GPU driver, so no Vulkan loader.
#   A. With no Vulkan loader, the one-click installer must stop with its message and install nothing.
#   B. With a Vulkan loader copied in (no GPU driver behind it), the installer must install,
#      keep the app-local MSVC runtime, open the app, and the app must stay up (CPU fallback).
#   C. The silent uninstaller must remove the app.
# Results: C:\LectusOut\result.json, screenshots and app logs. C:\LectusOut\done.json marks the end.
param([switch]$KeepOpen)
$ErrorActionPreference = 'Continue'
$in = 'C:\LectusIn'
$out = 'C:\LectusOut'
$sys = Join-Path $env:SystemRoot 'System32'
$runtime = @('msvcp140.dll', 'vcruntime140.dll', 'vcruntime140_1.dll', 'vcomp140.dll')
$result = [ordered]@{}
function Save-Result { $result | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $out 'result.json') -Encoding UTF8 }
function Note([string]$Key, $Value) { $result[$Key] = $Value; Save-Result }

Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class SB {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr h, int id);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, int m, IntPtr w, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
  public static List<IntPtr> MessageBoxes(uint pid) {
    var list = new List<IntPtr>();
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid && IsWindowVisible(h)) {
        var sb = new System.Text.StringBuilder(64); GetClassName(h, sb, 64);
        if (sb.ToString() == "#32770" && GetDlgItem(h, 0xFFFF) != IntPtr.Zero) list.Add(h);
      }
      return true;
    }, IntPtr.Zero);
    return list;
  }
  public static string Text(IntPtr h, int id) {
    IntPtr c = GetDlgItem(h, id); if (c == IntPtr.Zero) return "";
    var sb = new System.Text.StringBuilder(2048); GetWindowText(c, sb, 2048); return sb.ToString();
  }
}
'@

function Save-Screen([string]$Name) {
  try {
    $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
    $bmp.Save((Join-Path $out "$Name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
  } catch { }
}

function Find-Install {
  $keys = Get-ChildItem 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall' -ErrorAction SilentlyContinue
  foreach ($k in $keys) {
    $p = Get-ItemProperty -LiteralPath $k.PSPath -ErrorAction SilentlyContinue
    if ($p.DisplayName -like 'Lectus*') { return $p }
  }
  return $null
}

function Install-Dir($Entry) { if ($Entry) { return $Entry.InstallLocation.Trim('"') } return $null }

try {
  Note 'environment' ([ordered]@{
    os = [string](Get-CimInstance Win32_OperatingSystem).Caption + ' ' + [Environment]::OSVersion.Version
    systemRuntime = @($runtime | ForEach-Object { '{0}={1}' -f $_, (Test-Path (Join-Path $sys $_)) })
    vulkanLoader = Test-Path (Join-Path $sys 'vulkan-1.dll')
    webView2 = (Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue).pv
    screen = [string][System.Windows.Forms.Screen]::PrimaryScreen.Bounds
  })
  $setup = Join-Path $in 'setup.exe'

  # A. No Vulkan loader: the installer must stop before changing anything.
  if (-not (Test-Path (Join-Path $sys 'vulkan-1.dll'))) {
    $p = Start-Process -FilePath $setup -PassThru
    $text = $null
    $deadline = (Get-Date).AddSeconds(60)
    while (-not $p.HasExited -and -not $text -and (Get-Date) -lt $deadline) {
      foreach ($h in [SB]::MessageBoxes([uint32]$p.Id)) {
        Start-Sleep -Milliseconds 500
        Save-Screen 'a-vulkan-message'
        $text = [SB]::Text($h, 0xFFFF)
        [void][SB]::PostMessage($h, 0x0111, [IntPtr]2, [IntPtr]::Zero)  # a lone OK button has id 2
      }
      Start-Sleep -Milliseconds 250
    }
    [void]$p.WaitForExit(30000)
    Note 'a_noVulkan' ([ordered]@{
      message = $text
      exitCode = if ($p.HasExited) { $p.ExitCode } else { 'still running' }
      installedAnything = [bool](Find-Install)
    })
  } else {
    Note 'a_noVulkan' 'skipped: this sandbox already has a Vulkan loader'
  }

  # B. Vulkan loader present (no GPU driver behind it): install, auto-launch, stay up.
  $loader = Join-Path $in 'vulkan-1.dll'
  if (Test-Path $loader) { Copy-Item -LiteralPath $loader -Destination $sys -Force }
  $started = Get-Date
  $p = Start-Process -FilePath $setup -PassThru
  $shot = 0
  while (-not $p.HasExited -and ((Get-Date) - $started).TotalSeconds -lt 600) {
    if ($shot -lt 12) { Save-Screen ('b-install-{0:D2}' -f $shot); $shot++ }
    foreach ($h in [SB]::MessageBoxes([uint32]$p.Id)) {
      Save-Screen 'b-unexpected-message'
      Note 'b_unexpectedMessage' ([SB]::Text($h, 0xFFFF))
      [void][SB]::PostMessage($h, 0x0111, [IntPtr]2, [IntPtr]::Zero)
    }
    Start-Sleep -Seconds 1
  }
  $entry = Find-Install
  $dir = Install-Dir $entry
  $exeName = if ($entry) { $entry.MainBinaryName } else { $null }
  $base = if ($exeName) { [IO.Path]::GetFileNameWithoutExtension($exeName) } else { $null }
  $app = $null
  for ($i = 0; $i -lt 60 -and $base -and -not $app; $i++) { $app = Get-Process -Name $base -ErrorAction SilentlyContinue | Select-Object -First 1; if (-not $app) { Start-Sleep -Seconds 1 } }
  if ($app) { Start-Sleep -Seconds 20 }
  Save-Screen 'b-after-launch'
  $alive = $app -and -not $app.HasExited
  $modules = @()
  if ($alive) {
    $modules = @((Get-Process -Id $app.Id).Modules | Where-Object { $_.ModuleName -match '^(msvcp140|vcruntime140|vcruntime140_1|vcomp140|vulkan-1|ggml.*|llama.*)\.dll$' } | ForEach-Object { $_.FileName })
  }
  Note 'b_install' ([ordered]@{
    exitCode = if ($p.HasExited) { $p.ExitCode } else { 'still running' }
    seconds = [math]::Round(((Get-Date) - $started).TotalSeconds)
    displayVersion = if ($entry) { $entry.DisplayVersion } else { $null }
    publisher = if ($entry) { $entry.Publisher } else { $null }
    installDir = $dir
    appLocalRuntime = if ($dir) { @($runtime | Where-Object { Test-Path (Join-Path $dir $_) }) } else { @() }
    files = if ($dir) { @(Get-ChildItem -LiteralPath $dir | ForEach-Object Name) } else { @() }
    autoLaunched = [bool]$app
    aliveAfter20s = [bool]$alive
    loadedModules = $modules
  })

  # Relaunch with logs captured (the app logs to stderr) for a closer look.
  if ($dir -and $exeName) {
    Get-Process -Name $base -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
    $env:RUST_LOG = 'info'
    $q = Start-Process -FilePath (Join-Path $dir $exeName) -PassThru -RedirectStandardError (Join-Path $out 'app-stderr.log') -RedirectStandardOutput (Join-Path $out 'app-stdout.log')
    Start-Sleep -Seconds 25
    Save-Screen 'b-relaunch'
    Note 'b_relaunch' ([ordered]@{ aliveAfter25s = -not $q.HasExited; exitCode = if ($q.HasExited) { $q.ExitCode } else { $null } })
    Get-Process -Name $base -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
  }

  # C. Silent uninstall. `_?=` must reach NSIS unquoted, so pass one verbatim argument string.
  if ($dir) {
    $u = Start-Process -FilePath (Join-Path $dir 'uninstall.exe') -ArgumentList "/S _?=$dir" -PassThru -Wait
    Note 'c_uninstall' ([ordered]@{
      exitCode = $u.ExitCode
      registrationRemoved = -not [bool](Find-Install)
      left = @(Get-ChildItem -LiteralPath $dir -ErrorAction SilentlyContinue | ForEach-Object Name)
    })
  }
} catch {
  Note 'error' ([string]$_)
} finally {
  Note 'finished' (Get-Date).ToString('s')
  '{}' | Set-Content -LiteralPath (Join-Path $out 'done.json')
  if (-not $KeepOpen) { shutdown.exe /s /t 5 }
}
