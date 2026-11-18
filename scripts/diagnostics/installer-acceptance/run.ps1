# Windows guest only. Default performs read-only checks, never starts an installer.
[CmdletBinding()]
param(
    [ValidateSet('Preflight','CleanInstall','SeedData','Upgrade','InterruptUpgrade','Repair','UninstallPreserve','VerifyDeletedData')]
    [string]$Phase = 'Preflight',
    [string]$ArtifactDirectory = '',
    [string]$ManifestPath = '',
    [string]$ExpectedManifestSHA256 = '',
    [string]$GuestRoot = 'C:\LectusAcceptance',
    [int]$TimeoutSeconds = 180,
    # Silent = `/S`. OneClick = the double-click path (`/NOLAUNCH`, branded window,
    # no app launch). `/S` skips the one-click reinstall logic, so Upgrade and
    # Repair should also run once with OneClick.
    [ValidateSet('Silent','OneClick')]
    [string]$InstallMode = 'Silent'
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $ArtifactDirectory) { $ArtifactDirectory = $PSScriptRoot }
if (-not $ManifestPath) { $ManifestPath = Join-Path $PSScriptRoot 'artifacts.json' }
. (Join-Path $PSScriptRoot 'guards.ps1')

function Hash-File([string]$Path) {
    if (Test-Path -LiteralPath $Path -PathType Leaf) { return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash }
    return $null
}
function Assert-PlainPath([string]$Path) {
    $full = [IO.Path]::GetFullPath($Path)
    if ($full.StartsWith('\\') -or $full.Contains('"') -or $full -notmatch '^[A-Za-z]:\\') { throw 'Only local absolute drive paths are accepted.' }
    $cursor = $full
    while ($cursor) {
        if (Test-Path -LiteralPath $cursor) {
            if ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Reparse-point paths are refused.' }
        }
        $parent = Split-Path -Parent $cursor
        if ($parent -eq $cursor) { break }; $cursor = $parent
    }
    return $full
}
$GuestRoot = Assert-PlainPath $GuestRoot
$installDir = Assert-PlainPath (Join-Path $env:LOCALAPPDATA 'Lectus')
$dataDirs = @((Join-Path $env:APPDATA 'ai.organic.lectus'), (Join-Path $env:LOCALAPPDATA 'ai.organic.lectus')) | Select-Object -Unique
$manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
$manifestHash = Hash-File $ManifestPath
$computer = Get-CimInstance Win32_ComputerSystem
$machineGuid = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Cryptography' -Name MachineGuid).MachineGuid
$isVM = ($computer.Model -match 'Virtual Machine|VirtualBox|VMware|KVM|QEMU|Parallels|HVM domU')
$report = [ordered]@{
    schema = 1; timestampUTC = [DateTime]::UtcNow.ToString('o'); phase = $Phase
    manifestSHA256 = $manifestHash; virtualMachineDetected = $isVM
    isolatedGuestAttested = $false; installerExecuted = $false
    result = 'PreflightOnly'; checks = @(); executableSHA256 = (Hash-File (Join-Path $installDir 'chirp.exe'))
}
foreach ($role in @('baseline','candidate')) {
    $entry = $manifest.$role
    $complete = ($entry.installerFile -and $entry.installerSHA256 -match '^[A-Fa-f0-9]{64}$' -and $entry.executableSHA256 -match '^[A-Fa-f0-9]{64}$' -and $entry.uninstallerSHA256 -match '^[A-Fa-f0-9]{64}$' -and @($entry.installedFiles).Count -gt 0)
    $report.checks += [ordered]@{ artifact = $role; completeHashWhitelist = [bool]$complete; status = $entry.status }
}
if ($Phase -eq 'Preflight') { $report | ConvertTo-Json -Depth 8; return }

# HypervisorPresent alone also identifies physical hosts with WSL/VBS: never enough.
if (-not $isVM) { throw 'Refused: computer model is not a recognized disposable Windows guest. No installer was started.' }
if ($ExpectedManifestSHA256 -notmatch '^[A-Fa-f0-9]{64}$' -or $manifestHash -ne $ExpectedManifestSHA256) { throw 'Exact reviewed artifact-manifest SHA256 is required.' }
$attestationPath = Join-Path $GuestRoot 'environment.json'
$attestation = Get-Content -LiteralPath $attestationPath -Raw | ConvertFrom-Json
if ($attestation.disposable -ne $true -or $attestation.machineGuid -ne $machineGuid -or $attestation.computerName -ne $env:COMPUTERNAME -or -not $attestation.snapshotReference) { throw 'Disposable guest attestation does not match this machine/snapshot.' }
$report.isolatedGuestAttested = $true
foreach ($dataDir in $dataDirs) { $null = Assert-PlainPath $dataDir }
$runningApp = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -ieq 'chirp.exe' })
if ($runningApp.Count) { throw 'Close Lectus in this disposable guest first; runner never kills application processes.' }
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Lectus'
$statePath = Join-Path $GuestRoot 'sentinels.json'

function Get-Sentinels {
    if (-not (Test-Path -LiteralPath $statePath -PathType Leaf)) { throw 'SeedData receipt is required.' }
    # PS5 emits a top-level JSON array as one pipeline item. Assign before
    # normalizing so the receipt is eight objects rather than a nested array.
    $parsed = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $items = @($parsed)
    $expectedPaths = New-Object 'Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    foreach ($root in $dataDirs) {
        foreach ($name in @('config.json','history.json','api.json','models\acceptance-sentinel.bin')) { [void]$expectedPaths.Add((Join-Path $root $name)) }
    }
    if ($items.Count -ne $expectedPaths.Count) { throw 'The complete synthetic-data receipt is required.' }
    foreach ($item in $items) {
        if ($item.sha256 -notmatch '^[A-Fa-f0-9]{64}$' -or -not $expectedPaths.Remove([string]$item.path)) { throw 'Invalid, duplicate or unexpected sentinel entry.' }
    }
    if ($expectedPaths.Count) { throw 'Synthetic-data receipt is missing expected entries.' }
    return $items
}
function Assert-Sentinels([bool]$Deleted = $false) {
    foreach ($item in (Get-Sentinels)) {
        $allowed = $false
        foreach ($root in $dataDirs) { if ($item.path.StartsWith($root + '\', [StringComparison]::OrdinalIgnoreCase)) { $allowed = $true } }
        if (-not $allowed) { throw 'Sentinel path escapes the guest app data directories.' }
        $null = Assert-PlainPath $item.path
        if ($Deleted) { if (Test-Path -LiteralPath $item.path) { throw 'Synthetic user data remains after explicitly selected data removal.' } }
        elseif ((Hash-File $item.path) -ne $item.sha256) { throw 'Synthetic user data changed or disappeared.' }
    }
}
function Get-Artifact([string]$Role) {
    $entry = $manifest.$Role
    if (-not $entry.installerFile -or [IO.Path]::GetFileName($entry.installerFile) -ne $entry.installerFile -or $entry.installerSHA256 -notmatch '^[A-Fa-f0-9]{64}$' -or $entry.executableSHA256 -notmatch '^[A-Fa-f0-9]{64}$' -or $entry.uninstallerSHA256 -notmatch '^[A-Fa-f0-9]{64}$' -or @($entry.installedFiles).Count -eq 0) { throw "Incomplete $Role artifact whitelist. No installer will run." }
    foreach ($file in $entry.installedFiles) {
        if ($file.path -notmatch '^[A-Za-z0-9_ .-]+(\\[A-Za-z0-9_ .-]+)*$' -or @($file.path.Split('\') | Where-Object { $_ -eq '.' -or $_ -eq '..' -or $_.EndsWith('.') -or $_.EndsWith(' ') }).Count -or $file.sha256 -notmatch '^[A-Fa-f0-9]{64}$') { throw 'Invalid installed-file whitelist.' }
        $null = Assert-PlainPath (Join-Path $installDir $file.path)
    }
    $path = Assert-PlainPath (Join-Path $ArtifactDirectory $entry.installerFile)
    if ((Hash-File $path) -ne $entry.installerSHA256) { throw "Unexpected $Role installer bytes." }
    return $path
}
function Invoke-Installer([string]$Role, [bool]$Interrupt = $false) {
    $installer = Get-Artifact $Role
    $previous = Hash-File (Join-Path $installDir 'chirp.exe')
    if ($Interrupt -and $previous -eq $manifest.$Role.executableSHA256) { throw 'Interruption needs distinct baseline/candidate executable hashes.' }
    # No /R: do not launch the app, accept terms, enable autostart or use any account.
    # OneClick passes /NOLAUNCH for the same reason; its window stays visible.
    $report.installMode = $InstallMode
    $shellBefore = Get-ShellState
    if ($InstallMode -eq 'OneClick') {
        $process = Start-Process -FilePath $installer -ArgumentList '/NOLAUNCH' -PassThru
    } else {
        $process = Start-Process -FilePath $installer -ArgumentList '/S' -PassThru -WindowStyle Hidden
    }
    $report.installerExecuted = $true
    if ($Interrupt) {
        $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
        while (-not $process.HasExited -and [DateTime]::UtcNow -lt $deadline) {
            if ((Hash-File (Join-Path $installDir 'chirp.exe')) -eq $manifest.$Role.executableSHA256) {
                # Stop only the exact installer process created above, never name matching.
                $process.Kill(); $process.WaitForExit()
                $report.result = 'InstallerProcessInterruptedAfterExecutableReplacement'
                Assert-Sentinels
                return
            }
            Start-Sleep -Milliseconds 10; $process.Refresh()
        }
        if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
        throw 'Interruption point was not observed; restore the guest snapshot. No power-loss coverage claimed.'
    }
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) { $process.Kill(); $process.WaitForExit(); throw 'Installer timed out; restore guest snapshot before retrying.' }
    if ($process.ExitCode -ne 0) { throw "Installer exit code $($process.ExitCode)." }
    if ((Hash-File (Join-Path $installDir 'chirp.exe')) -ne $manifest.$Role.executableSHA256) { throw 'Installed executable does not match expected hash.' }
    foreach ($file in $manifest.$Role.installedFiles) {
        if ((Hash-File (Join-Path $installDir $file.path)) -ne $file.sha256) { throw 'Installed runtime/resource hash does not match artifact whitelist.' }
    }
    if (-not (Test-Path -LiteralPath $uninstallKey)) { throw 'Per-user uninstall registration missing.' }
    Assert-ExpectedSHA256File -Path (Join-Path $installDir 'uninstall.exe') -ExpectedSHA256 $manifest.$Role.uninstallerSHA256
    if ($InstallMode -eq 'OneClick') {
        # One-click overwrites in place: it must not run the old uninstaller,
        # which would drop the launch-at-login entry and the shortcuts.
        $shellAfter = Get-ShellState
        if ($shellBefore.runLectus -and $shellAfter.runLectus -ne $shellBefore.runLectus) { throw 'One-click install changed or removed the Lectus Run entry.' }
        if ($shellBefore.startMenuShortcut -and -not $shellAfter.startMenuShortcut) { throw 'One-click install removed the Start menu shortcut.' }
        $report.shellState = @{ before = $shellBefore; after = $shellAfter }
    }
}
function Get-ShellState {
    $run = Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue
    $value = if ($run -and $run.PSObject.Properties['Lectus']) { [string]$run.Lectus } else { $null }
    @{
        runLectus = $value
        startMenuShortcut = Test-Path -LiteralPath (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Lectus.lnk')
    }
}
function Assert-BaselineInstalled {
    Assert-ExpectedSHA256File -Path (Join-Path $installDir 'chirp.exe') -ExpectedSHA256 $manifest.baseline.executableSHA256
    Assert-ExpectedSHA256File -Path (Join-Path $installDir 'uninstall.exe') -ExpectedSHA256 $manifest.baseline.uninstallerSHA256
    foreach ($file in $manifest.baseline.installedFiles) {
        Assert-ExpectedSHA256File -Path (Join-Path $installDir $file.path) -ExpectedSHA256 $file.sha256
    }
    if (-not (Test-Path -LiteralPath $uninstallKey)) { throw 'Verified baseline registration is required.' }
}
# Every mutating phase must reject an incomplete whitelist before any write,
# including receipt creation in finally. Missing file/hash can never equal a
# verified baseline. Retain both reviewed packages for all lifecycle branches.
foreach ($role in @('baseline','candidate')) { $null = Get-Artifact $role }
$receiptDir = Assert-PlainPath (Join-Path $GuestRoot 'receipts')
$null = Assert-PlainPath $statePath
try {
    switch ($Phase) {
        'CleanInstall' {
            if ((Test-Path -LiteralPath $installDir) -or (Test-Path -LiteralPath $uninstallKey)) { throw 'Clean guest required: install path or uninstall key already exists.' }
            foreach ($root in $dataDirs) { if (Test-Path -LiteralPath $root) { throw 'Clean guest required: app data already exists.' } }
            Invoke-Installer 'baseline'; $report.result = 'BaselineInstalled'
        }
        'SeedData' {
            Assert-BaselineInstalled
            if (Test-Path -LiteralPath $statePath) { throw 'Existing sentinel receipt will not be overwritten.' }
            $sentinels = @()
            foreach ($root in $dataDirs) {
                if (Test-Path -LiteralPath $root) { throw 'Existing guest app data will not be overwritten.' }
            }
            foreach ($root in $dataDirs) {
                $null = New-Item -ItemType Directory -Path (Join-Path $root 'models') -Force
                $payloads = @{
                    'config.json' = '{"cloud_api_key":"LECTUS_SYNTHETIC_NOT_A_REAL_KEY","use_cloud":false,"onboarding_completed":false}'
                    'history.json' = '[{"text":"LECTUS_SYNTHETIC_HISTORY"}]'
                    'api.json' = '{"settings":{"enabled":false},"token":"LECTUS_SYNTHETIC_NOT_A_REAL_TOKEN"}'
                    'models\acceptance-sentinel.bin' = 'LECTUS_SYNTHETIC_MODEL_NOT_LOADABLE'
                }
                foreach ($name in $payloads.Keys) {
                    $path = Join-Path $root $name
                    [IO.File]::WriteAllText($path, $payloads[$name], (New-Object Text.UTF8Encoding $false))
                    $sentinels += @{ path = $path; sha256 = (Hash-File $path) }
                }
            }
            $sentinels | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $statePath -Encoding UTF8
            $report.result = 'SyntheticDataSeeded'
        }
        'Upgrade' { Assert-BaselineInstalled; Assert-Sentinels; Invoke-Installer 'candidate'; Assert-Sentinels; $report.result = 'UpgradePreservedSyntheticData' }
        'InterruptUpgrade' { Assert-BaselineInstalled; Assert-Sentinels; Invoke-Installer 'candidate' $true }
        'Repair' { Assert-Sentinels; Invoke-Installer 'candidate'; Assert-Sentinels; $report.result = 'ReinstallRecoveredWhitelistedPayloadAndPreservedSyntheticData' }
        'UninstallPreserve' {
            Assert-Sentinels
            $null = Get-Artifact 'candidate'
            if ((Hash-File (Join-Path $installDir 'chirp.exe')) -ne $manifest.candidate.executableSHA256) { throw 'Verified candidate installation is required.' }
            foreach ($file in $manifest.candidate.installedFiles) {
                if ((Hash-File (Join-Path $installDir $file.path)) -ne $file.sha256) { throw 'Candidate payload changed before uninstall.' }
            }
            $uninstaller = Assert-PlainPath (Join-Path $installDir 'uninstall.exe')
            Assert-ExpectedSHA256File -Path $uninstaller -ExpectedSHA256 $manifest.candidate.uninstallerSHA256
            $report.uninstallerSHA256 = Hash-File $uninstaller
            $process = Start-Process -FilePath $uninstaller -ArgumentList '/S',("_?=" + $installDir) -PassThru -WindowStyle Hidden
            $report.installerExecuted = $true
            if (-not $process.WaitForExit($TimeoutSeconds * 1000)) { throw 'Uninstall timed out; inspect disposable guest.' }
            if ($process.ExitCode -ne 0 -or (Test-Path -LiteralPath (Join-Path $installDir 'chirp.exe')) -or (Test-Path -LiteralPath $uninstallKey)) { throw 'Uninstall did not remove executable/registration.' }
            $run = Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue
            if ($run -and $run.PSObject.Properties['Lectus']) { throw 'Lectus autostart registration remains.' }
            Assert-Sentinels; $report.result = 'UninstallPreservedSyntheticData'
        }
        'VerifyDeletedData' {
            if ((Test-Path -LiteralPath (Join-Path $installDir 'chirp.exe')) -or (Test-Path -LiteralPath $uninstallKey)) { throw 'Complete interactive uninstall with remove-data checkbox first.' }
            Assert-Sentinels $true; $report.result = 'ExplicitInteractiveDataRemovalObserved'
        }
    }
} catch { $report.result = 'Failed'; $report.error = $_.Exception.Message; throw }
finally {
    $report.executableSHA256 = Hash-File (Join-Path $installDir 'chirp.exe')
    $null = Assert-PlainPath $receiptDir
    $null = New-Item -ItemType Directory -Path $receiptDir -Force
    $receipt = Join-Path $receiptDir (([DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ')) + '-' + $Phase + '.json')
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $receipt -Encoding UTF8
    $report | ConvertTo-Json -Depth 8
}
