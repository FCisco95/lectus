param([ValidateRange(500, 60000)][int]$ActivationTimeoutMs = 500)
$ErrorActionPreference = 'Stop'
$binary = Join-Path $PSScriptRoot 'target/debug/lectus-insertion-diagnostic.exe'
if (!(Test-Path -LiteralPath $binary)) { throw 'Build this standalone fixture first with cargo build --offline --manifest-path scripts/diagnostics/insertion/Cargo.toml.' }
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class LectusOwnedInsertionWindow {
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
}
'@
$previous = [LectusOwnedInsertionWindow]::GetForegroundWindow()
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Lectus isolated insertion ' + [guid]::NewGuid().ToString('N')
$form.Size = New-Object System.Drawing.Size(420, 160)
$form.StartPosition = 'CenterScreen'
$form.TopMost = $true
$box = New-Object System.Windows.Forms.TextBox
$box.Multiline = $true
$box.Dock = 'Fill'
$form.Controls.Add($box)
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 50
$script:fixtureChild = $null
$script:fixtureFailure = $null
$script:fixturePassed = $false
$script:fixtureOutput = ''
$script:started = [DateTime]::UtcNow
$form.Add_Shown({
  $form.Activate()
  [LectusOwnedInsertionWindow]::SetForegroundWindow($form.Handle) | Out-Null
  $box.Focus() | Out-Null
  $timer.Start()
})
$timer.Add_Tick({
  if (!$script:fixtureChild) {
    # Let the owned form finish its Shown/activation messages. Never force
    # input or synthesize Alt if Windows refuses foreground ownership.
    if ([LectusOwnedInsertionWindow]::GetForegroundWindow() -ne $form.Handle) {
      if (([DateTime]::UtcNow - $script:started).TotalMilliseconds -gt $ActivationTimeoutMs) {
        $script:fixtureFailure = 'Owned fixture could not safely acquire foreground; no input sent.'
        $timer.Stop(); $form.Close()
      }
      return
    }
    $box.Focus() | Out-Null
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = $binary
    $info.Arguments = "$($form.Handle.ToInt64()) $($box.Handle.ToInt64()) $PID"
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.WindowStyle = 'Hidden'
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $script:fixtureChild = [Diagnostics.Process]::Start($info)
    return
  }
  if ($script:fixtureChild -and $script:fixtureChild.HasExited) {
    $timer.Stop()
    $output = $script:fixtureChild.StandardOutput.ReadToEnd()
    $errorText = $script:fixtureChild.StandardError.ReadToEnd()
    $expected = 'Lectus: ol' + [char]0xE1 + ', English, ' + [char]0x6C49 + [char]0x5B57 + '.'
    $script:fixturePassed = $script:fixtureChild.ExitCode -eq 0 -and $box.Text -ceq $expected
    if (!$script:fixturePassed) { $script:fixtureFailure = 'Insertion fixture failed or aborted: ' + $errorText }
    else { $script:fixtureOutput = $output.Trim() + "`nowned_winforms_text_matches=true pt_accents=true english=true chinese=true" }
    # Return focus only while it still belongs to this exact owned window.
    if ([LectusOwnedInsertionWindow]::GetForegroundWindow() -eq $form.Handle) { [LectusOwnedInsertionWindow]::SetForegroundWindow($previous) | Out-Null }
    $form.Close()
  } elseif (([DateTime]::UtcNow - $script:started).TotalSeconds -gt 5) {
    $timer.Stop()
    if ($script:fixtureChild -and !$script:fixtureChild.HasExited) { $script:fixtureChild.Kill() }
    $script:fixtureFailure = 'Owned insertion fixture timed out; only its own child was stopped.'
    $form.Close()
  }
})
try { $form.ShowDialog() | Out-Null }
finally {
  $timer.Stop(); $timer.Dispose(); $form.Dispose()
  if ($script:fixtureChild) { if (!$script:fixtureChild.HasExited) { $script:fixtureChild.Kill() }; $script:fixtureChild.Dispose() }
}
if (!$script:fixturePassed) { throw $script:fixtureFailure }
Write-Output $script:fixtureOutput
