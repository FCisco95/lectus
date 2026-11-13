# Definitions only: safe to load for focused guard checks without running the kit.
function Assert-ExpectedSHA256File {
    param([string]$Path, [string]$ExpectedSHA256)
    if ($ExpectedSHA256 -notmatch '^[A-Fa-f0-9]{64}$') { throw 'A nonempty exact SHA256 is required.' }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw 'The expected file is absent.' }
    $actualSHA256 = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
    if (-not $actualSHA256 -or $actualSHA256 -ne $ExpectedSHA256) { throw 'File bytes do not match the exact expected SHA256.' }
}
