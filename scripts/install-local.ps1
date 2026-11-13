param([switch]$Apply, [switch]$Restart, [string]$ExpectedSHA256, [string]$ExpectedManifestSHA256,
  [string]$RecoverBackup, [string]$ExpectedRecoveryManifestSHA256, [switch]$LoadFunctionsOnly)
$ErrorActionPreference = 'Stop'

function Resolve-ContainedPath {
  param([string]$Root, [string]$Relative)
  $base = [IO.Path]::GetFullPath($Root).TrimEnd([char[]]'\/')
  if ([IO.Path]::IsPathRooted($Relative)) { throw 'An install entry must be relative.' }
  # Reject aliases before GetFullPath normalizes them. Otherwise duplicate and
  # reserved-directory checks can be bypassed with sub/../backups or slashes.
  foreach ($segment in ($Relative -split '[\\/]')) {
    if (!$segment -or $segment -in @('.', '..') -or $segment -match '[ .]$|[:*?"<>|]' -or $segment -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)') { throw 'Install paths must use unambiguous relative segments.' }
  }
  $target = [IO.Path]::GetFullPath((Join-Path $base $Relative))
  if (!$target.StartsWith($base + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'An install entry escaped its declared root.' }
  $probe = $target
  while ($probe.Length -ge $base.Length) {
    if (Test-Path -LiteralPath $probe) {
      if ((Get-Item -LiteralPath $probe -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Install operations do not follow junctions or symbolic links.' }
    }
    if ($probe -eq $base) { break }
    $probe = [IO.Path]::GetDirectoryName($probe)
  }
  return $target
}

function Get-PlainFiles {
  param([string]$Directory)
  if ((Get-Item -LiteralPath $Directory -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Package resource roots cannot be junctions or symbolic links.' }
  foreach ($item in Get-ChildItem -LiteralPath $Directory -Force) {
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Package resources cannot contain junctions or symbolic links.' }
    if ($item.PSIsContainer) { Get-PlainFiles -Directory $item.FullName } else { $item }
  }
}

function Get-LocalInstallPlan {
  param([string]$RepositoryRoot, [string]$ReleaseRoot)
  $repoRoot = [IO.Path]::GetFullPath($RepositoryRoot).TrimEnd([char[]]'\/')
  $releaseRoot = [IO.Path]::GetFullPath($ReleaseRoot).TrimEnd([char[]]'\/')
  $entries = New-Object 'System.Collections.Generic.List[object]'
  $seen = @{}
  function Add-InstallEntry {
    param([string]$SourcePath, [string]$RelativePath)
    $relative = $RelativePath.Replace('/', '\')
    [void](Resolve-ContainedPath -Root $repoRoot -Relative $relative)
    if ($relative -match '^(?i:backups)(\\|$)') { throw 'The backup directory is reserved.' }
    if ($seen.ContainsKey($relative)) { throw "Duplicate package entry: $relative" }
    $absolute = [IO.Path]::GetFullPath($SourcePath)
    if (!$absolute.StartsWith($repoRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Package source escaped the repository.' }
    [void](Resolve-ContainedPath -Root $repoRoot -Relative $absolute.Substring($repoRoot.Length + 1))
    if (!(Test-Path -LiteralPath $absolute -PathType Leaf)) { throw "Missing package resource: $relative" }
    $seen[$relative] = $true
    $entries.Add([pscustomobject]@{ RelativePath=$relative; SourcePath=$absolute; SHA256=(Get-FileHash -LiteralPath $absolute -Algorithm SHA256).Hash })
  }
  Add-InstallEntry (Join-Path $releaseRoot 'chirp.exe') 'chirp.exe'
  # Match Tauri's complete resource map rather than copying arbitrary DLLs.
  $tauriRoot = Join-Path $repoRoot 'src-tauri'
  foreach ($configName in @('tauri.conf.json', 'tauri.windows.conf.json')) {
    $config = Get-Content -LiteralPath (Join-Path $tauriRoot $configName) -Raw | ConvertFrom-Json
    foreach ($resource in $config.bundle.resources.PSObject.Properties) {
      if ($resource.Name.Contains('*')) {
        if ($resource.Name -ne 'target/release/build/llama-cpp-sys-2-*/out/backends/*.dll') { throw 'Unsupported wildcard resource; review the install manifest.' }
        continue
      }
      $sourcePath = [IO.Path]::GetFullPath((Join-Path $tauriRoot $resource.Name))
      if (Test-Path -LiteralPath $sourcePath -PathType Container) {
        foreach ($file in Get-PlainFiles -Directory $sourcePath) {
          Add-InstallEntry $file.FullName (Join-Path ([string]$resource.Value) $file.FullName.Substring($sourcePath.TrimEnd([char[]]'\/').Length + 1))
        }
      } else { Add-InstallEntry $sourcePath ([string]$resource.Value) }
    }
  }
  # Only backends from the native build matching the runtime can be installed.
  $runtimeHash = (Get-FileHash -LiteralPath (Join-Path $releaseRoot 'llama.dll') -Algorithm SHA256).Hash
  $candidates = @(Get-ChildItem -LiteralPath (Join-Path $releaseRoot 'build') -Directory -Filter 'llama-cpp-sys-2-*' | Where-Object {
    $out = Join-Path $_.FullName 'out'
    (Test-Path -LiteralPath (Join-Path $out 'backends/ggml-vulkan.dll')) -and
      @(Get-PlainFiles -Directory $out | Where-Object { $_.Name -eq 'llama.dll' -and (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash -eq $runtimeHash }).Count -gt 0
  })
  if ($candidates.Count -ne 1) { throw 'Cannot identify one backend build matching release/llama.dll.' }
  foreach ($file in Get-ChildItem -LiteralPath (Join-Path $candidates[0].FullName 'out/backends') -File -Filter '*.dll') { Add-InstallEntry $file.FullName ('backends/' + $file.Name) }
  return @($entries | Sort-Object RelativePath)
}

function Get-InstallManifestHash {
  param([object[]]$Files)
  $text = (@($Files | Sort-Object RelativePath | ForEach-Object { $_.RelativePath.Replace('\', '/') + ':' + $_.SHA256 }) -join "`n")
  $hasher = [Security.Cryptography.SHA256]::Create()
  try { return ([BitConverter]::ToString($hasher.ComputeHash([Text.Encoding]::UTF8.GetBytes($text)))).Replace('-', '') }
  finally { $hasher.Dispose() }
}

function New-ContainedParent {
  param([string]$Root, [string]$RelativeFile, [System.Collections.Generic.List[string]]$Created)
  $target = Resolve-ContainedPath -Root $Root -Relative $RelativeFile
  $parent = [IO.Path]::GetDirectoryName($target)
  $missing = New-Object 'System.Collections.Generic.List[string]'
  while (!(Test-Path -LiteralPath $parent)) { $missing.Add($parent); $parent = [IO.Path]::GetDirectoryName($parent) }
  for ($index = $missing.Count - 1; $index -ge 0; $index--) {
    [void][IO.Directory]::CreateDirectory($missing[$index])
    if ($null -ne $Created) { $Created.Add($missing[$index]) }
  }
  return $target
}

function Write-DurableInstallJson {
  param([string]$Path, [object]$Value)
  $temporary = $Path + '.' + [guid]::NewGuid().ToString('N') + '.tmp'
  $bytes = [Text.Encoding]::UTF8.GetBytes((ConvertTo-Json -InputObject $Value -Depth 8))
  $stream = [IO.FileStream]::new($temporary, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None, 4096, [IO.FileOptions]::WriteThrough)
  try { $stream.Write($bytes, 0, $bytes.Length); $stream.Flush($true) } finally { $stream.Dispose() }
  if (Test-Path -LiteralPath $Path) { [IO.File]::Replace($temporary, $Path, [NullString]::Value) }
  else { [IO.File]::Move($temporary, $Path) }
}

function Copy-AtomicInstallFile {
  param([string]$Source, [string]$Destination, [string]$BackupRoot, [string]$SHA256)
  # The temporary promotion stays on the destination volume, outside installed
  # resource paths. Interruption cannot leave a half-written installed file.
  $temporary = Resolve-ContainedPath -Root $BackupRoot -Relative ('promotion-' + [guid]::NewGuid().ToString('N') + '.tmp')
  try {
    Copy-Item -LiteralPath $Source -Destination $temporary
    if ((Get-FileHash -LiteralPath $temporary -Algorithm SHA256).Hash -ne $SHA256) { throw 'Promotion bytes failed verification.' }
    $stream = [IO.FileStream]::new($temporary, [IO.FileMode]::Open, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $stream.Flush($true) } finally { $stream.Dispose() }
    if (Test-Path -LiteralPath $Destination -PathType Leaf) { [IO.File]::Replace($temporary, $Destination, [NullString]::Value) }
    else { [IO.File]::Move($temporary, $Destination) }
  } finally { if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Force } }
}

function Open-LocalInstallLock {
  param([string]$DestinationRoot)
  if (!(Test-Path -LiteralPath $DestinationRoot -PathType Container)) { throw 'An existing destination directory is required.' }
  $lockPath = Resolve-ContainedPath -Root $DestinationRoot -Relative '.lectus-install.lock'
  try { return [IO.FileStream]::new($lockPath, [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None) }
  catch { throw 'Another install or recovery is active, or its exclusive lock cannot be opened. No installed files were changed.' }
}

function Assert-NoPendingLocalInstall {
  param([string]$DestinationRoot)
  $destinationRoot = [IO.Path]::GetFullPath($DestinationRoot).TrimEnd([char[]]'\/')
  $backupDirectory = Join-Path $destinationRoot 'backups'
  if (!(Test-Path -LiteralPath $backupDirectory)) { return }
  [void](Resolve-ContainedPath -Root $destinationRoot -Relative 'backups/validation')
  foreach ($prior in Get-ChildItem -LiteralPath $backupDirectory -Directory) {
    $priorJournal = Resolve-ContainedPath -Root $destinationRoot -Relative ('backups/' + $prior.Name + '/transaction.json')
    if (!(Test-Path -LiteralPath $priorJournal)) { continue } # staging never reached the write phase
    $state = Get-Content -LiteralPath $priorJournal -Raw | ConvertFrom-Json
    if ($state.Status -eq 'Installing') { throw 'A previous installation was interrupted. Preview its backup and recover manually before retrying. No process was stopped or relaunched.' }
    if ($state.Schema -ne 1 -or $state.DestinationRoot -ne $destinationRoot -or $state.Status -notin @('Committed', 'RolledBack') -or $state.ManifestSHA256 -notmatch '^[A-Fa-f0-9]{64}$') { throw 'An installation journal is invalid. Preserve its backup for manual recovery; no process was stopped or relaunched.' }
    $manifest = Resolve-ContainedPath -Root $destinationRoot -Relative ('backups/' + $prior.Name + '/manifest.json')
    if ((Get-FileHash -LiteralPath $manifest -Algorithm SHA256).Hash -ne $state.ManifestSHA256) { throw 'An installation journal manifest changed. Preserve it for manual review; no process was stopped or relaunched.' }
  }
}

function Invoke-WithLocalInstallLock {
  param([string]$DestinationRoot, [scriptblock]$Action, [switch]$RequireCleanState)
  # Acquisition is outside the action, including its stop/restart error handler.
  # A busy caller must never stop or relaunch a process owned by another run.
  $lock = Open-LocalInstallLock -DestinationRoot $DestinationRoot
  try {
    # This gate runs before entering the action's process/restart error handler.
    if ($RequireCleanState) { Assert-NoPendingLocalInstall -DestinationRoot $DestinationRoot }
    & $Action
  } finally { $lock.Dispose() }
}

function Invoke-LocalInstallRecovery {
  param([string]$DestinationRoot, [string]$BackupRoot, [string]$ExpectedRecoveryManifestSHA256)
  $lock = Open-LocalInstallLock -DestinationRoot $DestinationRoot
  try { Invoke-LocalInstallRecoveryCore @PSBoundParameters } finally { $lock.Dispose() }
}

function Invoke-LocalInstallRecoveryCore {
  param([string]$DestinationRoot, [string]$BackupRoot, [string]$ExpectedRecoveryManifestSHA256)
  $destinationRoot = [IO.Path]::GetFullPath($DestinationRoot).TrimEnd([char[]]'\/')
  $backupRoot = [IO.Path]::GetFullPath($BackupRoot).TrimEnd([char[]]'\/')
  if (!$backupRoot.StartsWith($destinationRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Recovery backup must be inside the install root.' }
  [void](Resolve-ContainedPath -Root $destinationRoot -Relative $backupRoot.Substring($destinationRoot.Length + 1))
  $manifestPath = Resolve-ContainedPath -Root $backupRoot -Relative 'manifest.json'
  $statePath = Resolve-ContainedPath -Root $backupRoot -Relative 'transaction.json'
  if (!$ExpectedRecoveryManifestSHA256 -or (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash -ne $ExpectedRecoveryManifestSHA256) { throw 'Preview recovery first and provide its exact ExpectedRecoveryManifestSHA256.' }
  $state = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
  if ($state.Schema -ne 1 -or $state.DestinationRoot -ne $destinationRoot -or $state.ManifestSHA256 -ne $ExpectedRecoveryManifestSHA256 -or $state.Status -notin @('Installing', 'Committed', 'RolledBack')) { throw 'Invalid recovery journal or destination.' }
  if ($state.Status -eq 'Committed') { return [pscustomobject]@{ Recovered=$false; Status='Committed'; Backup=$backupRoot } }
  $parsedRecords = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
  $records = @($parsedRecords)
  $seen = @{}
  # Validate every path, original backup and current file before the first write.
  # Unknown current bytes may be a later user edit: preserve them and refuse.
  foreach ($record in $records) {
    $destination = Resolve-ContainedPath -Root $destinationRoot -Relative $record.RelativePath
    $canonical = $record.RelativePath.Replace('/', '\')
    if ($seen.ContainsKey($canonical) -or $canonical -match '^(?i:backups)(\\|$)|^(?i:\.lectus-install\.lock)$' -or $record.SHA256 -notmatch '^[A-Fa-f0-9]{64}$') { throw 'Invalid recovery entry.' }
    $seen[$canonical] = $true
    if (Test-Path -LiteralPath $destination -PathType Container) { throw 'Recovery entry conflicts with a directory.' }
    if ($record.Existed) {
      $original = Resolve-ContainedPath -Root $backupRoot -Relative ('original/' + $record.RelativePath)
      if ($record.OldSHA256 -notmatch '^[A-Fa-f0-9]{64}$' -or (Get-FileHash -LiteralPath $original -Algorithm SHA256).Hash -ne $record.OldSHA256) { throw 'Recovery original backup failed verification; no files were changed.' }
    }
    if (Test-Path -LiteralPath $destination) {
      $hash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash
      if ($hash -ne $record.SHA256 -and (!$record.Existed -or $hash -ne $record.OldSHA256)) { throw 'Recovery found bytes outside this transaction; preserve them for manual review.' }
    } elseif ($record.Existed) { throw 'Recovery found a missing original file; preserve the backup for manual review.' }
  }
  foreach ($record in $records) {
    $destination = Resolve-ContainedPath -Root $destinationRoot -Relative $record.RelativePath
    if ($record.Existed) {
      Copy-AtomicInstallFile -Source (Resolve-ContainedPath $backupRoot ('original/' + $record.RelativePath)) -Destination $destination -BackupRoot $backupRoot -SHA256 $record.OldSHA256
      if ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash -ne $record.OldSHA256) { throw 'Recovered file hash mismatch.' }
    } elseif (Test-Path -LiteralPath $destination) { Remove-Item -LiteralPath $destination -Force }
  }
  $state.Status = 'RolledBack'
  Write-DurableInstallJson -Path $statePath -Value $state
  return [pscustomobject]@{ Recovered=$true; Status='RolledBack'; Backup=$backupRoot }
}

function Invoke-LocalFileInstall {
  param([object[]]$Files, [string]$DestinationRoot, [string]$BackupRoot, [scriptblock]$BeforeCopy)
  $lock = Open-LocalInstallLock -DestinationRoot $DestinationRoot
  try { Invoke-LocalFileInstallCore @PSBoundParameters } finally { $lock.Dispose() }
}

function Invoke-LocalFileInstallCore {
  param([object[]]$Files, [string]$DestinationRoot, [string]$BackupRoot, [scriptblock]$BeforeCopy)
  $destinationRoot = [IO.Path]::GetFullPath($DestinationRoot).TrimEnd([char[]]'\/')
  $backupRoot = [IO.Path]::GetFullPath($BackupRoot).TrimEnd([char[]]'\/')
  if (!(Test-Path -LiteralPath $destinationRoot -PathType Container)) { throw 'An existing destination directory is required.' }
  if (!$backupRoot.StartsWith($destinationRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Backups must be inside the declared install root.' }
  [void](Resolve-ContainedPath -Root $destinationRoot -Relative $backupRoot.Substring($destinationRoot.Length + 1))
  if (Test-Path -LiteralPath $backupRoot) { throw 'Choose a new backup directory for this transaction.' }
  Assert-NoPendingLocalInstall -DestinationRoot $destinationRoot
  $seenFiles = @{}
  foreach ($file in $Files) {
    $target = Resolve-ContainedPath -Root $destinationRoot -Relative $file.RelativePath
    $canonical = $file.RelativePath.Replace('/', '\')
    if ($seenFiles.ContainsKey($canonical) -or $canonical -match '^(?i:backups)(\\|$)|^(?i:\.lectus-install\.lock)$' -or $file.SHA256 -notmatch '^[A-Fa-f0-9]{64}$') { throw 'Invalid or duplicate package entry.' }
    $seenFiles[$canonical] = $true
    if (Test-Path -LiteralPath $target -PathType Container) { throw 'A package file conflicts with an existing directory; no installed files were replaced.' }
    if ((Get-FileHash -LiteralPath $file.SourcePath -Algorithm SHA256).Hash -ne $file.SHA256) { throw 'A package source changed after the manifest was reviewed.' }
  }
  [void][IO.Directory]::CreateDirectory($backupRoot)
  $records = New-Object 'System.Collections.Generic.List[object]'
  # Freeze candidate bytes and back up all originals before the first overwrite.
  foreach ($file in $Files) {
    $destination = Resolve-ContainedPath -Root $destinationRoot -Relative $file.RelativePath
    $existed = Test-Path -LiteralPath $destination -PathType Leaf
    $oldHash = $null
    if ($existed) {
      $original = New-ContainedParent -Root $backupRoot -RelativeFile ('original/' + $file.RelativePath)
      $oldHash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash
      Copy-Item -LiteralPath $destination -Destination $original
      if ((Get-FileHash -LiteralPath $original -Algorithm SHA256).Hash -ne $oldHash) { throw 'Could not verify an installation backup.' }
      $stream = [IO.FileStream]::new($original, [IO.FileMode]::Open, [IO.FileAccess]::Write, [IO.FileShare]::None)
      try { $stream.Flush($true) } finally { $stream.Dispose() }
    }
    $staged = New-ContainedParent -Root $backupRoot -RelativeFile ('candidate/' + $file.RelativePath)
    Copy-Item -LiteralPath $file.SourcePath -Destination $staged
    if ((Get-FileHash -LiteralPath $staged -Algorithm SHA256).Hash -ne $file.SHA256) { throw 'The package changed while staging. No installed files were replaced.' }
    $records.Add([pscustomobject]@{ RelativePath=$file.RelativePath; Existed=$existed; OldSHA256=$oldHash; SHA256=$file.SHA256 })
  }
  $manifestPath = Join-Path $backupRoot 'manifest.json'
  Write-DurableInstallJson -Path $manifestPath -Value @($records.ToArray())
  $recoveryHash = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash
  $journal = [pscustomobject]@{ Schema=1; Status='Installing'; DestinationRoot=$destinationRoot; ManifestSHA256=$recoveryHash }
  $journalPath = Join-Path $backupRoot 'transaction.json'
  Write-DurableInstallJson -Path $journalPath -Value $journal
  $created = New-Object 'System.Collections.Generic.List[string]'
  try {
    foreach ($record in $records) {
      if ($BeforeCopy) { & $BeforeCopy $record.RelativePath }
      $destination = New-ContainedParent -Root $destinationRoot -RelativeFile $record.RelativePath -Created $created
      Copy-AtomicInstallFile -Source (Resolve-ContainedPath $backupRoot ('candidate/' + $record.RelativePath)) -Destination $destination -BackupRoot $backupRoot -SHA256 $record.SHA256
      if ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash -ne $record.SHA256) { throw 'Installed file does not match the reviewed manifest.' }
    }
    $journal.Status = 'Committed'
    Write-DurableInstallJson -Path $journalPath -Value $journal
  } catch {
    $originalFailure = $_
    $rollbackFailures = New-Object 'System.Collections.Generic.List[string]'
    try { Invoke-LocalInstallRecoveryCore -DestinationRoot $destinationRoot -BackupRoot $backupRoot -ExpectedRecoveryManifestSHA256 $recoveryHash | Out-Null }
    catch { $rollbackFailures.Add($_.Exception.Message) }
    foreach ($directory in @($created | Sort-Object Length -Descending)) {
      try {
        $relative = $directory.Substring($destinationRoot.Length + 1)
        $checked = Resolve-ContainedPath -Root $destinationRoot -Relative $relative
        if ((Test-Path -LiteralPath $checked -PathType Container) -and @(Get-ChildItem -LiteralPath $checked -Force).Count -eq 0) { Remove-Item -LiteralPath $checked }
      } catch { $rollbackFailures.Add($directory) }
    }
    if ($rollbackFailures.Count) {
      $failure = New-Object InvalidOperationException("Installation failed ($($originalFailure.Exception.Message)) and rollback needs attention for: $($rollbackFailures -join ', '). Backups: $backupRoot")
      $failure.Data['RollbackFailed'] = $true
      throw $failure
    }
    throw $originalFailure
  }
  return [pscustomobject]@{ Applied=$true; Backup=$backupRoot; ManifestSHA256=(Get-InstallManifestHash $Files); RecoveryManifestSHA256=$recoveryHash; Status='Committed' }
}

if ($LoadFunctionsOnly) { return }
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$source = Join-Path $repo 'src-tauri/target/release'
$install = Join-Path $env:LOCALAPPDATA 'Lectus'
$sourceExe = Join-Path $source 'chirp.exe'
$installedExe = Join-Path $install 'chirp.exe'
if ($RecoverBackup) {
  $recoveryRoot = [IO.Path]::GetFullPath($RecoverBackup)
  if (!$recoveryRoot.StartsWith(([IO.Path]::GetFullPath((Join-Path $install 'backups')) + [IO.Path]::DirectorySeparatorChar), [StringComparison]::OrdinalIgnoreCase)) { throw 'Choose a backup under the existing Lectus installation.' }
  [void](Resolve-ContainedPath -Root $install -Relative $recoveryRoot.Substring($install.Length + 1))
  $recoveryManifest = Resolve-ContainedPath -Root $recoveryRoot -Relative 'manifest.json'
  $recoveryHash = (Get-FileHash -LiteralPath $recoveryManifest -Algorithm SHA256).Hash
  if (!$Apply) { [pscustomobject]@{ Backup=$recoveryRoot; RecoveryManifestSHA256=$recoveryHash; Applied=$false; Journal=(Get-Content -LiteralPath (Join-Path $recoveryRoot 'transaction.json') -Raw | ConvertFrom-Json) } | ConvertTo-Json -Depth 5; return }
  Invoke-WithLocalInstallLock -DestinationRoot $install -Action {
    $active = @(Get-CimInstance Win32_Process -Filter "Name = 'chirp.exe'" | Where-Object { $_.ExecutablePath -eq $installedExe })
    if ($active.Count) { throw 'Finish dictation and exit Lectus before recovery; no process was stopped.' }
    Invoke-LocalInstallRecoveryCore -DestinationRoot $install -BackupRoot $recoveryRoot -ExpectedRecoveryManifestSHA256 $ExpectedRecoveryManifestSHA256 | ConvertTo-Json -Depth 4
  }
  return
}
if (!(Test-Path -LiteralPath $sourceExe) -or !(Test-Path -LiteralPath $installedExe)) { throw 'Both the release build and an existing Lectus installation are required.' }
$files = @(Get-LocalInstallPlan -RepositoryRoot $repo -ReleaseRoot $source)
$buildHash = (Get-FileHash -LiteralPath $sourceExe -Algorithm SHA256).Hash
$manifestHash = Get-InstallManifestHash $files
if ($ExpectedSHA256 -and $buildHash -ne $ExpectedSHA256) { throw 'The executable changed since it was reviewed.' }
if ($ExpectedManifestSHA256 -and $manifestHash -ne $ExpectedManifestSHA256) { throw 'The runtime or resources changed since the manifest was reviewed.' }
$running = @(Get-CimInstance Win32_Process -Filter "Name = 'chirp.exe'" | Where-Object { $_.ExecutablePath -eq $installedExe })
if (!$Apply) {
  [pscustomobject]@{ Source=$sourceExe; Destination=$installedExe; BuildSHA256=$buildHash; ManifestSHA256=$manifestHash;
    InstalledSHA256=(Get-FileHash -LiteralPath $installedExe -Algorithm SHA256).Hash; RunningPIDs=@($running.ProcessId);
    Files=@($files | Select-Object RelativePath,SHA256); Applied=$false } | ConvertTo-Json -Depth 4
  return
}
if (!$ExpectedManifestSHA256) { throw 'Preview the file manifest first, then apply with its ExpectedManifestSHA256.' }
$backup = Resolve-ContainedPath -Root $install -Relative ('backups/' + [guid]::NewGuid().ToString('N'))
Invoke-WithLocalInstallLock -DestinationRoot $install -RequireCleanState -Action {
try {
  # Refresh process state only after acquiring the lifecycle lock. The earlier
  # snapshot is for the read-only preview and may be stale after another run.
  $running = @(Get-CimInstance Win32_Process -Filter "Name = 'chirp.exe'" | Where-Object { $_.ExecutablePath -eq $installedExe })
  if ($running.Count -gt 0 -and !$Restart) { throw 'Lectus is running. Finish dictation before applying with -Restart.' }
  foreach ($process in $running) {
    $current = Get-CimInstance Win32_Process -Filter "ProcessId = $($process.ProcessId)"
    if ($current -and $current.ExecutablePath -eq $installedExe) {
      Stop-Process -Id $process.ProcessId
      Wait-Process -Id $process.ProcessId -Timeout 15 -ErrorAction SilentlyContinue
      $remaining = Get-CimInstance Win32_Process -Filter "ProcessId = $($process.ProcessId)"
      if ($remaining -and $remaining.ExecutablePath -eq $installedExe) { throw 'Lectus did not stop; no installed files were replaced.' }
    }
  }
  $result = Invoke-LocalFileInstallCore -Files $files -DestinationRoot $install -BackupRoot $backup
  $restarted = $false
  $launchError = $null
  if ($Restart) {
    try { Start-Process -FilePath $installedExe -WindowStyle Hidden -WorkingDirectory $install; $restarted = $true }
    catch { $launchError = 'Files were installed and verified, but Lectus could not be launched. Open it manually; the backup is retained.' }
  }
  $result | Add-Member -NotePropertyName Restarted -NotePropertyValue $restarted
  $result | Add-Member -NotePropertyName LaunchError -NotePropertyValue $launchError
  $result | ConvertTo-Json -Depth 3
} catch {
  $stillRunning = @(Get-CimInstance Win32_Process -Filter "Name = 'chirp.exe'" | Where-Object { $_.ExecutablePath -eq $installedExe })
  if ($Restart -and $running.Count -gt 0 -and $stillRunning.Count -eq 0 -and !$_.Exception.Data['RollbackFailed']) { Start-Process -FilePath $installedExe -WindowStyle Hidden -WorkingDirectory $install }
  throw
}
}
