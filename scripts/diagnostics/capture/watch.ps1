param(
  [ValidateRange(15, 90)][int]$WatchSeconds = 90,
  [string]$ReceiptPath = (Join-Path $PSScriptRoot ('receipts/' + [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ') + '.json'))
)
$ErrorActionPreference = 'Stop'
$binary = Join-Path $PSScriptRoot 'target/debug/lectus-capture-diagnostic.exe'
$installed = Join-Path $env:LOCALAPPDATA 'Lectus/chirp.exe'
$config = Join-Path $env:APPDATA 'ai.organic.lectus/config.json'
$expectedInstalledHash = 'CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406'
$destination = [IO.Path]::GetFullPath($ReceiptPath)
if ($destination.Contains('"')) { throw 'Invalid receipt path' }
if (!(Test-Path -LiteralPath $binary)) { throw 'Build the standalone diagnostic first with scripts/diagnostics/capture/build.cmd.' }
if (Test-Path -LiteralPath $destination) { throw 'Existing receipt will not be overwritten.' }
$app = @(Get-Process -Name chirp -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $installed })
if ($app.Count -ne 1) { throw 'Expected one already-running installed Lectus; no app was changed.' }
$appId = $app[0].Id
$appStart = $app[0].StartTime.ToUniversalTime()
$configHash = (Get-FileHash -LiteralPath $config -Algorithm SHA256).Hash
$installedHash = (Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash
if ($installedHash -ne $expectedInstalledHash) { throw 'Installed app differs from the reviewed baseline; no diagnostic started.' }
[void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($destination))
$info = New-Object Diagnostics.ProcessStartInfo
$info.FileName = $binary
$info.Arguments = '--watch-seconds ' + $WatchSeconds + ' --receipt "' + $destination + '"'
$info.UseShellExecute = $false; $info.CreateNoWindow = $true; $info.WindowStyle = 'Hidden'
$info.RedirectStandardOutput = $true; $info.RedirectStandardError = $true
$child = $null
$guardFailure = $null
$started = [DateTimeOffset]::UtcNow
Write-Output 'Diagnostic only: wait for ready_for_human_unplug_replug before removing USB. Installed Lectus may need manual reopening afterwards; this helper never restarts it.'
try {
  $child = [Diagnostics.Process]::Start($info)
  $stdout = $child.StandardOutput.ReadLineAsync()
  $stderr = $child.StandardError.ReadToEndAsync()
  while (!$child.HasExited) {
    if ($stdout.IsCompleted) {
      $line = $stdout.Result
      if ($null -ne $line) { Write-Output $line; $stdout = $child.StandardOutput.ReadLineAsync() }
    }
    $liveApp = Get-Process -Id $appId -ErrorAction SilentlyContinue
    if (!$liveApp -or $liveApp.Path -ne $installed -or $liveApp.StartTime.ToUniversalTime() -ne $appStart) {
      $guardFailure = 'Installed process identity changed; only the diagnostic child was stopped.'; break
    }
    if ((Get-FileHash -LiteralPath $config -Algorithm SHA256).Hash -ne $configHash) {
      $guardFailure = 'Configuration changed; only the diagnostic child was stopped.'; break
    }
    if (([DateTimeOffset]::UtcNow - $started).TotalSeconds -gt ($WatchSeconds + 20)) {
      $guardFailure = 'Diagnostic exceeded its physical-watch deadline; only its own child was stopped.'; break
    }
    Start-Sleep -Milliseconds 250
  }
  if ($guardFailure) { if (!$child.HasExited) { $child.Kill() }; throw $guardFailure }
  # Drain already-requested async line before reading the remaining lines.
  $lastLine = $stdout.GetAwaiter().GetResult()
  if ($null -ne $lastLine) { Write-Output $lastLine }
  $remaining = $child.StandardOutput.ReadToEnd()
  if ($remaining) { Write-Output $remaining.TrimEnd() }
  $nativeError = $stderr.GetAwaiter().GetResult()
  # Preserve a generic error only; device/API diagnostics can contain names.
  if ($nativeError) { Write-Output 'Native stream reported an error; inspect outcome/metrics in the privacy-limited receipt.' }
  if ((Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash -ne $installedHash) { throw 'Installed binary changed during observation.' }
  if ((Get-FileHash -LiteralPath $config -Algorithm SHA256).Hash -ne $configHash) { throw 'Configuration changed before final guard.' }
  if ($child.ExitCode -ne 0) { throw 'Native diagnostic aborted; inspect its privacy-limited receipt if written.' }
  Write-Output ('installed_process_unchanged=true installed_sha256_unchanged=true config_sha256_unchanged=true receipt=' + $destination)
} finally {
  if ($child) { if (!$child.HasExited) { $child.Kill() }; $child.Dispose() }
}
