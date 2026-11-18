# Authenticode-sign one file for a Windows release build.
#
# Tauri calls this for chirp.exe, the NSIS installer and its uninstaller (see
# src-tauri/tauri.signing.conf.json, merged only into release builds with
# `tauri build --config src-tauri/tauri.signing.conf.json`). Signing happens
# before Tauri writes the updater signature, so both stay valid.
#
# Environment:
#   LECTUS_SIGN_CERT_SHA1      thumbprint of the code-signing certificate in the
#                              current user's store (for Certum SimplySign, the
#                              certificate shown while SimplySign Desktop is
#                              connected). Required.
#   LECTUS_SIGN_TIMESTAMP_URL  RFC 3161 timestamp server. Default: Certum's.
#                              "none" skips timestamping (local tests only).
param([Parameter(Mandatory)] [string]$File)
$ErrorActionPreference = 'Stop'

$thumbprint = $env:LECTUS_SIGN_CERT_SHA1
if (-not $thumbprint) {
  throw 'LECTUS_SIGN_CERT_SHA1 is not set. Release builds must be signed; see scripts/sign-windows.ps1.'
}
$thumbprint = $thumbprint -replace '\s', ''
$timestamp = if ($env:LECTUS_SIGN_TIMESTAMP_URL) { $env:LECTUS_SIGN_TIMESTAMP_URL } else { 'http://time.certum.pl' }

# Newest signtool from the Windows SDK.
$signtool = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
  Sort-Object { [version]($_.Directory.Parent.Name) } -Descending | Select-Object -First 1
if (-not $signtool) { throw 'signtool.exe not found. Install the Windows 10/11 SDK signing tools.' }

$arguments = @('sign', '/sha1', $thumbprint, '/fd', 'sha256', '/d', 'Lectus')
if ($timestamp -ne 'none') { $arguments += @('/tr', $timestamp, '/td', 'sha256') }
$arguments += $File

& $signtool.FullName @arguments
if ($LASTEXITCODE -ne 0) { throw "signtool failed ($LASTEXITCODE) for $File" }
