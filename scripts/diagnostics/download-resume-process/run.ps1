param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$context = Join-Path $repo '.context'
[void][IO.Directory]::CreateDirectory($context)
$target = Join-Path $context 'download-resume-process-target'
$fixture = Join-Path $context ('download-resume-process-' + [guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($fixture)
$executable = Join-Path $target 'release/lectus-download-resume-process.exe'
if (!$SkipBuild) {
  & cargo build --release --offline --locked --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml') --target-dir $target
  if ($LASTEXITCODE -ne 0) { throw 'Standalone diagnostic compilation failed.' }
}
if (!(Test-Path -LiteralPath $executable -PathType Leaf)) { throw 'Diagnostic executable not found.' }
$server = $null
$first = $null
$second = $null
function Start-DiagnosticProcess {
  param([string[]]$Arguments, [string]$Name)
  # All variable paths are generated beneath this workspace, with no quotes in
  # their names. Start-Process ArgumentList needs explicit quoting for spaces.
  $quoted = @($Arguments | ForEach-Object { if ($_.Contains('"')) { throw 'Unexpected quote in a diagnostic argument.' }; '"' + $_ + '"' })
  $process = Start-Process -FilePath $executable -ArgumentList $quoted -WindowStyle Hidden -PassThru `
    -RedirectStandardOutput (Join-Path $fixture ($Name + '.stdout')) -RedirectStandardError (Join-Path $fixture ($Name + '.stderr'))
  # Retain the OS process handle before exit. Windows PowerShell can otherwise
  # lose ExitCode when a short-lived redirected child has already disappeared.
  [void]$process.Handle
  return $process
}
function Wait-Condition {
  param([scriptblock]$Condition, [int]$Seconds, [string]$Failure)
  $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
  do { if (& $Condition) { return }; Start-Sleep -Milliseconds 20 } while ([DateTime]::UtcNow -lt $deadline)
  throw $Failure
}
try {
  $server = Start-DiagnosticProcess -Arguments @('serve', $fixture) -Name 'server'
  $serverInfoPath = Join-Path $fixture 'server.json'
  Wait-Condition { Test-Path -LiteralPath $serverInfoPath } 5 'Fixture server did not become ready.'
  $info = Get-Content -LiteralPath $serverInfoPath -Raw | ConvertFrom-Json
  $destination = Join-Path $fixture 'resumed-model.bin'
  $partial = [IO.Path]::ChangeExtension($destination, 'part')
  $receipt = [IO.Path]::ChangeExtension($destination, 'part.json')
  $first = Start-DiagnosticProcess -Arguments @('download', $info.url, $destination, $info.sha256) -Name 'first'
  Wait-Condition { (Test-Path -LiteralPath $partial) -and (Test-Path -LiteralPath $receipt) -and (Get-Item -LiteralPath $partial).Length -ge 131072 } 10 'First downloader did not save partial bytes and receipt.'
  if (Test-Path -LiteralPath $destination) { throw 'Incomplete first download was published.' }
  # Terminate exactly the child process we launched. No app process or service
  # is enumerated, closed or replaced, and no cancel token is used.
  if ($first.HasExited) { throw 'First child completed before the intended termination.' }
  Stop-Process -Id $first.Id -Force
  $first.WaitForExit()
  $saved = (Get-Item -LiteralPath $partial).Length
  if ($saved -le 0 -or $saved -ge $info.size) { throw 'Termination did not leave an incomplete partial.' }
  if (Test-Path -LiteralPath $destination) { throw 'Killed process published an incomplete model.' }
  $metadata = Get-Content -LiteralPath $receipt -Raw | ConvertFrom-Json
  if ($metadata.checksum -ne $info.sha256 -or $metadata.total -ne $info.size) { throw 'Saved receipt did not survive termination.' }
  $second = Start-DiagnosticProcess -Arguments @('download', $info.url, $destination, $info.sha256) -Name 'second'
  if (!$second.WaitForExit(15000)) { throw 'Restarted downloader did not finish.' }
  if ($second.ExitCode -ne 0) { throw ('Restarted downloader failed: ' + (Get-Content -LiteralPath (Join-Path $fixture 'second.stderr') -Raw)) }
  if (!$server.WaitForExit(5000)) { throw 'Fixture server did not finish.' }
  if ($server.ExitCode -ne 0) { throw ('Fixture server failed: ' + (Get-Content -LiteralPath (Join-Path $fixture 'server.stderr') -Raw)) }
  $completed = Get-Content -LiteralPath (Join-Path $fixture 'second.stdout') -Raw | ConvertFrom-Json
  $requests = @(Get-Content -LiteralPath (Join-Path $fixture 'requests.jsonl') | ForEach-Object { $_ | ConvertFrom-Json })
  $hash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($completed.resumed_from -ne $saved -or $requests.Count -ne 2 -or $requests[1].offset -ne $saved) { throw 'Restart did not resume at the exact saved offset.' }
  if ($hash -ne $info.sha256 -or (Get-Item -LiteralPath $destination).Length -ne $info.size) { throw 'Final model failed integrity verification.' }
  if ((Test-Path -LiteralPath $partial) -or (Test-Path -LiteralPath $receipt)) { throw 'Completed publication left partial state behind.' }
  $result = [pscustomobject]@{
    Passed=$true; TimestampUtc=[DateTime]::UtcNow.ToString('o'); FixtureRoot=$fixture
    ProductionTransportSHA256=(Get-FileHash -LiteralPath (Join-Path $repo 'src-tauri/src/model_download.rs') -Algorithm SHA256).Hash
    KilledChildPID=$first.Id; SavedBytes=$saved; ResumedFrom=$completed.resumed_from; FinalBytes=$info.size; SHA256=$hash
    Evidence='Standalone production transport, actual child termination and new process, loopback HTTP; native app shutdown not exercised.'
  }
  $result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $fixture 'result.json') -Encoding utf8
  $result | ConvertTo-Json -Depth 5
} finally {
  foreach ($process in @($first, $second, $server)) {
    if ($null -ne $process) { $process.Refresh(); if (!$process.HasExited) { Stop-Process -Id $process.Id -Force; $process.WaitForExit() } }
  }
}
