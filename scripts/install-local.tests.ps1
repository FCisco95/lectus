$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'install-local.ps1') -LoadFunctionsOnly
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$fixtureRelative = '.context/install-local-tests-' + [guid]::NewGuid().ToString('N')
$fixtureRoot = Resolve-ContainedPath -Root $workspace -Relative $fixtureRelative
[void][IO.Directory]::CreateDirectory($fixtureRoot)
$passed = 0
function Assert-True { param([bool]$Value, [string]$Message) if (!$Value) { throw $Message } }
function Write-Fixture {
  param([string]$Relative, [string]$Text)
  $path = New-ContainedParent -Root $fixtureRoot -RelativeFile $Relative
  [IO.File]::WriteAllText($path, $Text)
}
function Expect-Failure {
  param([scriptblock]$Action)
  $failed = $false
  try { & $Action | Out-Null } catch { $failed = $true }
  Assert-True $failed 'Expected the fixture action to fail.'
}
try {
  $repo = Join-Path $fixtureRoot 'repo'
  $release = Join-Path $repo 'src-tauri/target/release'
  Write-Fixture 'repo/src-tauri/target/release/chirp.exe' 'candidate executable'
  foreach ($dll in @('ggml.dll','ggml-base.dll','llama.dll','llama-common.dll')) { Write-Fixture ('repo/src-tauri/target/release/' + $dll) ('runtime-' + $dll) }
  Write-Fixture 'repo/src-tauri/target/release/stale.dll' 'must not be installed'
  Write-Fixture 'repo/src-tauri/target/release/build/llama-cpp-sys-2-matching/out/bin/llama.dll' 'runtime-llama.dll'
  Write-Fixture 'repo/src-tauri/target/release/build/llama-cpp-sys-2-matching/out/backends/ggml-vulkan.dll' 'matching backend'
  Write-Fixture 'repo/src-tauri/target/release/build/llama-cpp-sys-2-matching/out/backends/ggml-cpu.dll' 'matching cpu'
  Write-Fixture 'repo/src-tauri/target/release/build/llama-cpp-sys-2-old/out/bin/llama.dll' 'different old runtime'
  Write-Fixture 'repo/src-tauri/target/release/build/llama-cpp-sys-2-old/out/backends/ggml-vulkan.dll' 'wrong backend'
  Write-Fixture 'repo/legal/TERMS.md' 'fixture terms'
  Write-Fixture 'repo/legal/third-party/dependency/LICENSE' 'fixture dependency license'
  Write-Fixture 'repo/models/ggml-tiny.en.bin' 'fixture fallback model'
  Write-Fixture 'repo/src-tauri/icons/tray-idle.png' 'fixture icon'
  $resources = @{ '../legal/TERMS.md'='legal/TERMS.md'; '../legal/third-party/'='legal/third-party/'; '../models/ggml-tiny.en.bin'='models/ggml-tiny.en.bin'; 'icons/tray-idle.png'='icons/tray-idle.png' }
  Write-Fixture 'repo/src-tauri/tauri.conf.json' (@{bundle=@{resources=$resources}} | ConvertTo-Json -Depth 5)
  $windows = @{ 'target/release/build/llama-cpp-sys-2-*/out/backends/*.dll'='backends/' }
  foreach ($dll in @('ggml.dll','ggml-base.dll','llama.dll','llama-common.dll')) { $windows['target/release/' + $dll]=$dll }
  Write-Fixture 'repo/src-tauri/tauri.windows.conf.json' (@{bundle=@{resources=$windows}} | ConvertTo-Json -Depth 5)
  $plan = @(Get-LocalInstallPlan -RepositoryRoot $repo -ReleaseRoot $release)
  $paths = @($plan.RelativePath)
  Assert-True ($paths -contains 'legal\third-party\dependency\LICENSE') 'Third-party notices were omitted.'
  Assert-True ($paths -contains 'models\ggml-tiny.en.bin') 'The fallback resource was omitted.'
  Assert-True ($paths -contains 'backends\ggml-vulkan.dll') 'Matched native backends were omitted.'
  Assert-True (!($paths -contains 'stale.dll')) 'An arbitrary stale DLL entered the candidate.'
  $passed++

  Write-Fixture 'success/chirp.exe' 'old executable'
  Write-Fixture 'success/personal-note.txt' 'unrelated file'
  $success = Join-Path $fixtureRoot 'success'
  $result = Invoke-LocalFileInstall -Files $plan -DestinationRoot $success -BackupRoot (Join-Path $success 'backups/test')
  Assert-True $result.Applied 'Fixture installation did not complete.'
  foreach ($file in $plan) { Assert-True ((Get-FileHash -LiteralPath (Join-Path $success $file.RelativePath)).Hash -eq $file.SHA256) 'Installed resource differs from the frozen manifest.' }
  Assert-True ((Get-Content -LiteralPath (Join-Path $success 'personal-note.txt') -Raw) -eq 'unrelated file') 'Unrelated files changed.'
  Assert-True ((Get-Content -LiteralPath (Join-Path $success 'backups/test/original/chirp.exe') -Raw) -eq 'old executable') 'Original executable backup missing.'
  $committedRecovery = Invoke-LocalInstallRecovery -DestinationRoot $success -BackupRoot $result.Backup -ExpectedRecoveryManifestSHA256 $result.RecoveryManifestSHA256
  Assert-True ($committedRecovery.Status -eq 'Committed' -and !$committedRecovery.Recovered) 'Recovery undid a committed transaction.'
  $passed++

  # Clean payload deployment does not touch the separate app-data directory.
  # These are fixtures, not a claim that the NSIS wizard was exercised.
  foreach ($name in @('config.json','history.json','credentials.bin','models/user-model.bin')) { Write-Fixture ('user-data/' + $name) ('private fixture sentinel ' + $name) }
  $sentinels = @(Get-PlainFiles -Directory (Join-Path $fixtureRoot 'user-data') | ForEach-Object { [pscustomobject]@{ Path=$_.FullName; Hash=(Get-FileHash -LiteralPath $_.FullName).Hash } })
  $clean = Join-Path $fixtureRoot 'clean'
  [void][IO.Directory]::CreateDirectory($clean)
  $cleanResult = Invoke-LocalFileInstall -Files $plan -DestinationRoot $clean -BackupRoot (Join-Path $clean 'backups/test')
  Assert-True $cleanResult.Applied 'Clean payload deployment failed.'
  foreach ($sentinel in $sentinels) { Assert-True ((Get-FileHash -LiteralPath $sentinel.Path).Hash -eq $sentinel.Hash) 'Deployment changed app data.' }
  $passed++

  Write-Fixture 'rollback/chirp.exe' 'old executable'
  Write-Fixture 'rollback/legal/TERMS.md' 'old terms'
  $rollback = Join-Path $fixtureRoot 'rollback'
  Expect-Failure { Invoke-LocalFileInstall -Files $plan -DestinationRoot $rollback -BackupRoot (Join-Path $rollback 'backups/test') -BeforeCopy { param($relative) if ($relative -eq 'models\ggml-tiny.en.bin') { throw 'Injected copy failure' } } }
  Assert-True ((Get-Content -LiteralPath (Join-Path $rollback 'chirp.exe') -Raw) -eq 'old executable') 'Rollback did not restore the executable.'
  Assert-True ((Get-Content -LiteralPath (Join-Path $rollback 'legal/TERMS.md') -Raw) -eq 'old terms') 'Rollback did not restore the existing resource.'
  foreach ($file in $plan | Where-Object { $_.RelativePath -notin @('chirp.exe','legal\TERMS.md') }) { Assert-True (!(Test-Path -LiteralPath (Join-Path $rollback $file.RelativePath))) 'Rollback left a newly introduced file behind.' }
  Assert-True (!(Test-Path -LiteralPath (Join-Path $rollback 'backends'))) 'Rollback left an empty created directory.'
  $passed++

  # Exit the installer child process abruptly, bypassing its catch/finally.
  # This reproduces process interruption, not a physical power-loss guarantee.
  Write-Fixture 'crash/chirp.exe' 'old executable'
  Write-Fixture 'crash/legal/TERMS.md' 'old terms'
  $crash = Join-Path $fixtureRoot 'crash'
  $crashBackup = Join-Path $crash 'backups/test'
  $planPath = Join-Path $fixtureRoot 'crash-plan.json'
  Write-DurableInstallJson -Path $planPath -Value $plan
  $childPath = Join-Path $fixtureRoot 'crash-child.ps1'
  [IO.File]::WriteAllText($childPath, @'
param($FunctionsPath, $PlanPath, $Destination, $Backup)
$ErrorActionPreference = 'Stop'
. $FunctionsPath -LoadFunctionsOnly
$parsed = Get-Content -LiteralPath $PlanPath -Raw | ConvertFrom-Json
Invoke-LocalFileInstall -Files @($parsed) -DestinationRoot $Destination -BackupRoot $Backup -BeforeCopy {
  param($relative)
  if ($relative -eq 'models\ggml-tiny.en.bin') { [Environment]::Exit(73) }
}
exit 74
'@)
  $hostExecutable = (Get-Process -Id $PID).Path
  & $hostExecutable -NoProfile -File $childPath -FunctionsPath (Join-Path $PSScriptRoot 'install-local.ps1') -PlanPath $planPath -Destination $crash -Backup $crashBackup
  Assert-True ($LASTEXITCODE -eq 73) 'The child did not terminate at the intended crash point.'
  Assert-True ((Get-Content -LiteralPath (Join-Path $crash 'chirp.exe') -Raw) -eq 'candidate executable') 'The interruption did not occur after replacing a file.'
  Assert-True ((Get-Content -LiteralPath (Join-Path $crashBackup 'transaction.json') -Raw | ConvertFrom-Json).Status -eq 'Installing') 'Interrupted transaction was not recorded persistently.'
  Expect-Failure { Invoke-LocalFileInstall -Files $plan -DestinationRoot $crash -BackupRoot (Join-Path $crash 'backups/retry') }
  $pendingAction = {
    try {
      Write-Fixture 'crash/unexpected-pending-stop' 'stop'
      throw 'Simulated failure after stopping'
    } catch { Write-Fixture 'crash/unexpected-pending-restart' 'restart'; throw }
  }
  Expect-Failure { Invoke-WithLocalInstallLock -DestinationRoot $crash -RequireCleanState -Action $pendingAction }
  Assert-True (!(Test-Path -LiteralPath (Join-Path $crash 'unexpected-pending-stop')) -and !(Test-Path -LiteralPath (Join-Path $crash 'unexpected-pending-restart'))) 'A pending journal entered the stop/restart error handler.'
  $journalText = Get-Content -LiteralPath (Join-Path $crashBackup 'transaction.json') -Raw
  Write-Fixture 'crash/backups/test/transaction.json' '{broken journal'
  Expect-Failure { Invoke-WithLocalInstallLock -DestinationRoot $crash -RequireCleanState -Action $pendingAction }
  Assert-True (!(Test-Path -LiteralPath (Join-Path $crash 'unexpected-pending-stop')) -and !(Test-Path -LiteralPath (Join-Path $crash 'unexpected-pending-restart'))) 'A corrupt journal entered the stop/restart error handler.'
  Write-Fixture 'crash/backups/test/transaction.json' $journalText
  Assert-True ((Get-Content -LiteralPath (Join-Path $crash 'chirp.exe') -Raw) -eq 'candidate executable') 'Pending-journal rejection changed the interrupted installation.'
  $passed++
  $recoveryHash = (Get-FileHash -LiteralPath (Join-Path $crashBackup 'manifest.json')).Hash
  Expect-Failure { Invoke-LocalInstallRecovery -DestinationRoot $crash -BackupRoot $crashBackup -ExpectedRecoveryManifestSHA256 ('0' * 64) }
  Assert-True ((Get-Content -LiteralPath (Join-Path $crash 'chirp.exe') -Raw) -eq 'candidate executable') 'Rejected recovery changed a file.'
  $passed++

  $recovered = Invoke-LocalInstallRecovery -DestinationRoot $crash -BackupRoot $crashBackup -ExpectedRecoveryManifestSHA256 $recoveryHash
  Assert-True $recovered.Recovered 'Explicit crash recovery failed.'
  Assert-True ((Get-Content -LiteralPath (Join-Path $crash 'chirp.exe') -Raw) -eq 'old executable') 'Crash recovery did not restore the executable.'
  Assert-True ((Get-Content -LiteralPath (Join-Path $crash 'legal/TERMS.md') -Raw) -eq 'old terms') 'Crash recovery did not restore resources.'
  foreach ($file in $plan | Where-Object { $_.RelativePath -notin @('chirp.exe','legal\TERMS.md') }) { Assert-True (!(Test-Path -LiteralPath (Join-Path $crash $file.RelativePath))) 'Crash recovery left a newly deployed file.' }
  Invoke-LocalInstallRecovery -DestinationRoot $crash -BackupRoot $crashBackup -ExpectedRecoveryManifestSHA256 $recoveryHash | Out-Null
  Assert-True ((Get-Content -LiteralPath (Join-Path $crash 'chirp.exe') -Raw) -eq 'old executable') 'Repeated recovery was not idempotent.'
  foreach ($sentinel in $sentinels) { Assert-True ((Get-FileHash -LiteralPath $sentinel.Path).Hash -eq $sentinel.Hash) 'Interruption/recovery changed app data.' }
  $passed++

  Write-Fixture 'crash/chirp.exe' 'later independent edit'
  Expect-Failure { Invoke-LocalInstallRecovery -DestinationRoot $crash -BackupRoot $crashBackup -ExpectedRecoveryManifestSHA256 $recoveryHash }
  Assert-True ((Get-Content -LiteralPath (Join-Path $crash 'chirp.exe') -Raw) -eq 'later independent edit') 'Recovery overwrote unknown user bytes.'
  Write-Fixture 'crash/chirp.exe' 'old executable'
  Write-Fixture 'crash/backups/test/original/chirp.exe' 'tampered backup'
  Expect-Failure { Invoke-LocalInstallRecovery -DestinationRoot $crash -BackupRoot $crashBackup -ExpectedRecoveryManifestSHA256 $recoveryHash }
  Assert-True ((Get-Content -LiteralPath (Join-Path $crash 'chirp.exe') -Raw) -eq 'old executable') 'Recovery copied an unverified backup.'
  $passed++

  Write-Fixture 'locked/chirp.exe' 'old executable'
  $locked = Join-Path $fixtureRoot 'locked'
  $concurrentPath = Join-Path $fixtureRoot 'concurrent-child.ps1'
  [IO.File]::WriteAllText($concurrentPath, @'
param($FunctionsPath, $PlanPath, $Destination)
$ErrorActionPreference = 'Stop'
. $FunctionsPath -LoadFunctionsOnly
$parsed = Get-Content -LiteralPath $PlanPath -Raw | ConvertFrom-Json
$blocked = 0
try { Invoke-LocalFileInstall -Files @($parsed) -DestinationRoot $Destination -BackupRoot (Join-Path $Destination 'backups/concurrent') | Out-Null }
catch { if ($_.Exception.Message -like '*exclusive lock*') { $blocked++ } else { throw } }
try { Invoke-LocalInstallRecovery -DestinationRoot $Destination -BackupRoot (Join-Path $Destination 'backups/test') -ExpectedRecoveryManifestSHA256 ('0' * 64) | Out-Null }
catch { if ($_.Exception.Message -like '*exclusive lock*') { $blocked++ } else { throw } }
try {
  Invoke-WithLocalInstallLock -DestinationRoot $Destination -Action {
    # Model the CLI's stop + failed-install + catch/restart lifecycle. A busy
    # contender must never enter this action or its internal error handler.
    [IO.File]::WriteAllText((Join-Path $Destination 'unexpected-stop'), 'stop')
    try { throw 'fixture install failure' }
    catch { [IO.File]::WriteAllText((Join-Path $Destination 'unexpected-restart'), 'restart'); throw }
  }
} catch { if ($_.Exception.Message -like '*exclusive lock*') { $blocked++ } else { throw } }
if ($blocked -eq 3) { exit 0 }
exit 75
'@)
  Invoke-WithLocalInstallLock -DestinationRoot $locked -RequireCleanState -Action {
    [IO.File]::AppendAllText((Join-Path $locked 'lifecycle.txt'), "stop`n")
    Invoke-LocalFileInstallCore -Files $plan -DestinationRoot $locked -BackupRoot (Join-Path $locked 'backups/test') -BeforeCopy {
      param($relative)
      if ($relative -eq 'chirp.exe') {
        & $hostExecutable -NoProfile -File $concurrentPath -FunctionsPath (Join-Path $PSScriptRoot 'install-local.ps1') -PlanPath $planPath -Destination $locked
        Assert-True ($LASTEXITCODE -eq 0) 'Another process bypassed the install/recovery/lifecycle lock.'
      }
    } | Out-Null
    [IO.File]::AppendAllText((Join-Path $locked 'lifecycle.txt'), "restart`n")
  }
  Assert-True (!(Test-Path -LiteralPath (Join-Path $locked 'backups/concurrent'))) 'The concurrent installer created a transaction.'
  Assert-True (!(Test-Path -LiteralPath (Join-Path $locked 'unexpected-stop')) -and !(Test-Path -LiteralPath (Join-Path $locked 'unexpected-restart'))) 'The busy CLI entered stop/restart actions.'
  Assert-True ((Get-Content -LiteralPath (Join-Path $locked 'lifecycle.txt') -Raw) -ceq "stop`nrestart`n") 'The primary lifecycle did not retain exclusive ownership.'
  $passed++

  foreach ($alias in @('sub/../backups/poison', 'sub/../chirp.exe', 'legal//TERMS.md', 'legal/TERMS.md.', '.lectus-install.lock')) {
    $badEntry = [pscustomobject]@{ RelativePath=$alias; SourcePath=$plan[0].SourcePath; SHA256=$plan[0].SHA256 }
    Expect-Failure { Invoke-LocalFileInstall -Files @($badEntry) -DestinationRoot $locked -BackupRoot (Join-Path $locked ('backups/alias-' + [guid]::NewGuid().ToString('N'))) }
  }
  $aliasEntries = @(
    [pscustomobject]@{ RelativePath='legal/extra'; SourcePath=$plan[0].SourcePath; SHA256=$plan[0].SHA256 },
    [pscustomobject]@{ RelativePath='legal\extra'; SourcePath=$plan[0].SourcePath; SHA256=$plan[0].SHA256 }
  )
  Expect-Failure { Invoke-LocalFileInstall -Files $aliasEntries -DestinationRoot $locked -BackupRoot (Join-Path $locked 'backups/duplicate') }
  $aliasRecovery = Join-Path $fixtureRoot 'alias-recovery'
  $aliasBackup = Join-Path $aliasRecovery 'backups/test'
  [void][IO.Directory]::CreateDirectory($aliasBackup)
  foreach ($pathsToReject in @(@('sub/../backups/poison'), @('legal/extra', 'legal\extra'))) {
    $aliasRecords = @($pathsToReject | ForEach-Object { [pscustomobject]@{ RelativePath=$_; Existed=$false; SHA256=$plan[0].SHA256 } })
    Write-DurableInstallJson -Path (Join-Path $aliasBackup 'manifest.json') -Value $aliasRecords
    $aliasHash = (Get-FileHash -LiteralPath (Join-Path $aliasBackup 'manifest.json')).Hash
    Write-DurableInstallJson -Path (Join-Path $aliasBackup 'transaction.json') -Value ([pscustomobject]@{ Schema=1; Status='Installing'; DestinationRoot=$aliasRecovery; ManifestSHA256=$aliasHash })
    Expect-Failure { Invoke-LocalInstallRecovery -DestinationRoot $aliasRecovery -BackupRoot $aliasBackup -ExpectedRecoveryManifestSHA256 $aliasHash }
  }
  $passed++

  Write-Fixture 'blocked/chirp.exe' 'unchanged'
  [void][IO.Directory]::CreateDirectory((Join-Path $fixtureRoot 'blocked/ggml.dll'))
  $blocked = Join-Path $fixtureRoot 'blocked'
  Expect-Failure { Invoke-LocalFileInstall -Files $plan -DestinationRoot $blocked -BackupRoot (Join-Path $blocked 'backups/test') }
  Assert-True ((Get-Content -LiteralPath (Join-Path $blocked 'chirp.exe') -Raw) -eq 'unchanged') 'Directory collision changed the installed executable.'
  $passed++

  Write-Fixture 'outside.txt' 'outside sentinel'
  $bad = [pscustomobject]@{ RelativePath='../outside.txt'; SourcePath=$plan[0].SourcePath; SHA256=$plan[0].SHA256 }
  Expect-Failure { Invoke-LocalFileInstall -Files @($bad) -DestinationRoot $blocked -BackupRoot (Join-Path $blocked 'backups/traversal') }
  Assert-True ((Get-Content -LiteralPath (Join-Path $fixtureRoot 'outside.txt') -Raw) -eq 'outside sentinel') 'An operation escaped the declared root.'
  $passed++

  $before = Get-InstallManifestHash $plan
  Write-Fixture 'repo/legal/TERMS.md' 'changed after review'
  $changedPlan = @(Get-LocalInstallPlan -RepositoryRoot $repo -ReleaseRoot $release)
  Assert-True ((Get-InstallManifestHash $changedPlan) -ne $before) 'Manifest did not cover resource changes.'
  Write-Fixture 'changed/chirp.exe' 'unchanged'
  $changed = Join-Path $fixtureRoot 'changed'
  Expect-Failure { Invoke-LocalFileInstall -Files $plan -DestinationRoot $changed -BackupRoot (Join-Path $changed 'backups/test') }
  Assert-True ((Get-Content -LiteralPath (Join-Path $changed 'chirp.exe') -Raw) -eq 'unchanged') 'Changed source was installed.'
  $passed++
  Write-Output "PASS: $passed isolated installer checks; no app process or real installation was touched."
} finally {
  # This test creates only this GUID fixture. Check the resolved target before
  # the one recursive cleanup, using PowerShell throughout.
  $resolved = (Resolve-Path -LiteralPath $fixtureRoot).Path
  [void](Resolve-ContainedPath -Root $workspace -Relative $fixtureRelative)
  $allowed = [IO.Path]::GetFullPath((Join-Path $workspace '.context')) + [IO.Path]::DirectorySeparatorChar
  if (!$resolved.StartsWith($allowed, [StringComparison]::OrdinalIgnoreCase) -or $resolved -ne [IO.Path]::GetFullPath($fixtureRoot)) { throw 'Refusing fixture cleanup outside the declared workspace.' }
  Remove-Item -LiteralPath $resolved -Recurse -Force
}
