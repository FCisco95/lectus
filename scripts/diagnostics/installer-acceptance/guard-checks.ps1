# Focused safety regressions. Reads only runner/guard source; no NSIS or writes.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'guards.ps1')
$guardPath = Join-Path $PSScriptRoot 'guards.ps1'
$rejected = 0
foreach ($value in @($null, '', 'invalid', ('0' * 64))) {
    try { Assert-ExpectedSHA256File -Path 'C:\LectusAcceptance\absent-file-for-guard-check.exe' -ExpectedSHA256 $value }
    catch { $rejected++ }
}
if ($rejected -ne 4) { throw 'Expected missing/invalid hash or absent file to be refused.' }
Assert-ExpectedSHA256File -Path $guardPath -ExpectedSHA256 (Get-FileHash -LiteralPath $guardPath -Algorithm SHA256).Hash

$tokens = $null; $parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'run.ps1'), [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw 'Runner syntax errors.' }
& {
    foreach ($name in @('Get-Sentinels', 'Get-Artifact')) {
        $definition = $ast.FindAll({ param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name }, $true)[0]
        . ([scriptblock]::Create($definition.Extent.Text))
    }
    $dataDirs = @('C:\Synthetic\Roaming\ai.organic.lectus', 'C:\Synthetic\Local\ai.organic.lectus')
    $statePath = 'C:\Synthetic\sentinels.json'
    # These stubs exercise JSON/predicates only; no fixture files are created.
    function Test-Path { param($LiteralPath, $PathType) return $true }
    function Get-Content { param($LiteralPath, [switch]$Raw) return $script:sentinelJSON }
    $script:sentinelJSON = '[]'
    $emptyRejected = $false
    try { Get-Sentinels } catch { $emptyRejected = $true }
    $items = @()
    foreach ($root in $dataDirs) {
        foreach ($name in @('config.json', 'history.json', 'api.json', 'models\acceptance-sentinel.bin')) {
            $items += @{ path = (Join-Path $root $name); sha256 = ('a' * 64) }
        }
    }
    $script:sentinelJSON = ConvertTo-Json -InputObject $items
    $actual = Get-Sentinels
    if (@($actual).Count -ne 8 -or $actual[0].path -isnot [string]) { throw 'Complete receipt was not normalized into eight sentinel objects.' }
    $items[7] = $items[0]
    $script:sentinelJSON = ConvertTo-Json -InputObject $items
    $duplicateRejected = $false
    try { Get-Sentinels } catch { $duplicateRejected = $true }
    if (-not $emptyRejected -or -not $duplicateRejected) { throw 'Empty/duplicate sentinel receipt accepted.' }
    $manifest = [pscustomobject]@{ baseline = [pscustomobject]@{ installerFile = $null; installerSHA256 = $null; executableSHA256 = $null; uninstallerSHA256 = $null; installedFiles = @() } }
    $whitelistRejected = $false
    try { Get-Artifact 'baseline' } catch { $whitelistRejected = $_.Exception.Message -eq 'Incomplete baseline artifact whitelist. No installer will run.' }
    if (-not $whitelistRejected) { throw 'Incomplete artifact whitelist accepted.' }
}
'PASS: null/invalid hashes, absent file, complete-file accept, empty/duplicate/complete JSON sentinels, incomplete artifact whitelist; no writes or installer execution.'
