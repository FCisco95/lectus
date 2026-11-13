param(
  [ValidateRange(15, 180)][int]$TimeoutSeconds = 180,
  [string]$ReceiptPath = (Join-Path $PSScriptRoot ('receipts/' + [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ') + '.json')),
  [string]$ExpectedInstalledSHA256 = 'CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406'
)
$ErrorActionPreference = 'Stop'
# Interactive acceptance ONLY. This helper never sends keys, plays sound,
# captures audio, changes config, starts/stops Lectus, or reads other windows.
if ([Threading.Thread]::CurrentThread.ApartmentState -ne 'STA') { throw 'Use Windows PowerShell with -STA.' }
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class LectusHumanAcceptanceWindow {
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern IntPtr GetFocus();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
}
'@
$installed = Join-Path $env:LOCALAPPDATA 'Lectus/chirp.exe'
$configPath = Join-Path $env:APPDATA 'ai.organic.lectus/config.json'
$historyPath = Join-Path $env:APPDATA 'ai.organic.lectus/history.json'
$utf8 = New-Object Text.UTF8Encoding($false)
function Get-Hash([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash }
function Get-TextHash([string]$Text) {
  $sha = [Security.Cryptography.SHA256]::Create()
  try { ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($Text)))).Replace('-', '') }
  finally { $sha.Dispose() }
}
function Get-HistoryMetadata {
  if (!(Test-Path -LiteralPath $historyPath)) { return @{ count = 0; latest_timestamp_ms = 0 } }
  # Old text is never inspected, copied into the receipt, or printed.
  # Windows PowerShell 5.1 emits a JSON root array as one pipeline object.
  # Materialize first, then enumerate the variable so each entry is handled.
  $parsed = Get-Content -LiteralPath $historyPath -Raw -Encoding UTF8 | ConvertFrom-Json
  $entries = @($parsed)
  $latest = 0L
  foreach ($entry in $entries) { if ([long]$entry.timestamp -gt $latest) { $latest = [long]$entry.timestamp } }
  return @{ count = $entries.Count; latest_timestamp_ms = $latest }
}
function Get-Normalized([string]$Text) {
  $decomposed = $Text.ToLowerInvariant().Normalize([Text.NormalizationForm]::FormD)
  $builder = New-Object Text.StringBuilder
  foreach ($c in $decomposed.ToCharArray()) {
    if ([Globalization.CharUnicodeInfo]::GetUnicodeCategory($c) -ne [Globalization.UnicodeCategory]::NonSpacingMark) { [void]$builder.Append($c) }
  }
  return ([regex]::Replace($builder.ToString(), '[^a-z0-9]+', ' ')).Trim()
}
function Get-InstalledProcess {
  $matches = @(Get-Process -Name chirp -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $installed })
  if ($matches.Count -ne 1) { throw 'Expected exactly one already-running installed Lectus. No app was started or stopped.' }
  return $matches[0]
}
$configBefore = Get-Hash $configPath
$cfg = Get-Content -LiteralPath $configPath -Raw -Encoding UTF8 | ConvertFrom-Json
if (!$cfg.onboarding_completed -or $cfg.use_cloud -or $cfg.ai_cleanup_enabled -or @($cfg.app_profiles).Count -gt 0 -or
    ($null -ne $cfg.output_language -and $cfg.output_language -ne 'same') -or
    $cfg.hold_hotkey -ne 'RControl' -or $cfg.toggle_hotkey -ne 'F13' -or $cfg.trigger_mode -ne 'hold') {
  throw 'Current settings differ from the reviewed local-only human test; no configuration was changed.'
}
$cfg = $null
$installedBefore = Get-Hash $installed
if ($installedBefore -ne $ExpectedInstalledSHA256) { throw 'Installed hash differs from reviewed version; no app was started or stopped.' }
$app = Get-InstalledProcess
$appId = $app.Id
$appStart = $app.StartTime.ToUniversalTime()
$version = (Get-Item -LiteralPath $installed).VersionInfo.FileVersion
$historyBefore = Get-HistoryMetadata
$started = [DateTimeOffset]::UtcNow
$script:finished = $false
$script:outcome = 'pending'
$script:failure = $null
$script:textObservedAtMs = $null
$script:ownedFocusObserved = $false
$script:textOutsideOwnedFocus = $false
$script:historyAfter = $historyBefore
$script:configAfter = $configBefore
$script:installedAfter = $installedBefore
$script:keywords = @{}
$form = New-Object Windows.Forms.Form
$form.Text = 'Lectus - teste humano isolado ' + [guid]::NewGuid().ToString('N').Substring(0,8)
$form.Size = New-Object Drawing.Size(680, 330)
$form.StartPosition = 'CenterScreen'
$form.TopMost = $true
$form.AccessibleName = 'Campo isolado de teste humano do Lectus'
$label = New-Object Windows.Forms.Label
$label.Dock = 'Top'; $label.Height = 105
$label.Text = "Clique no campo abaixo. Segure o Ctrl DIREITO, diga:`r`nA reuni" + [char]0xE3 + 'o ' + [char]0xE9 + ' na quinta-feira ' + [char]0xE0 + "s dez horas`r`nSolte o Ctrl direito e aguarde. N" + [char]0xE3 + 'o escreva nem cole texto.'
$label.Font = New-Object Drawing.Font('Segoe UI', 12)
$box = New-Object Windows.Forms.TextBox
$box.Multiline = $true; $box.Dock = 'Fill'; $box.Font = New-Object Drawing.Font('Segoe UI', 14)
$box.AccessibleName = 'Dite a frase de teste aqui; nao escreva nem cole'
$status = New-Object Windows.Forms.Label
$status.Dock = 'Bottom'; $status.Height = 38; $status.Text = 'A aguardar foco no campo e ditado humano. Fechar cancela.'
$form.Controls.Add($box); $form.Controls.Add($label); $form.Controls.Add($status)
function Test-OwnedFocus {
  $foreground = [LectusHumanAcceptanceWindow]::GetForegroundWindow()
  $focused = [LectusHumanAcceptanceWindow]::GetFocus()
  [uint32]$owner = 0
  [void][LectusHumanAcceptanceWindow]::GetWindowThreadProcessId($foreground, [ref]$owner)
  return $foreground -eq $form.Handle -and $owner -eq $PID -and $focused -eq $box.Handle
}
function Finish-Test([string]$Outcome, [string]$Failure) {
  $script:finished = $true; $script:outcome = $Outcome; $script:failure = $Failure
  $timer.Stop(); $form.Close()
}
$box.Add_TextChanged({
  if ($box.Text.Length -eq 0) { return }
  if (!(Test-OwnedFocus)) { $script:textOutsideOwnedFocus = $true }
  $script:textObservedAtMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
})
$timer = New-Object Windows.Forms.Timer
$timer.Interval = 250
$form.Add_Shown({ $box.Focus() | Out-Null; $timer.Start() })
$timer.Add_Tick({
  try {
    $script:configAfter = Get-Hash $configPath
    if ($script:configAfter -ne $configBefore) { Finish-Test 'aborted' 'Configuration changed during acceptance. No synthetic input was sent.'; return }
    $currentApp = Get-InstalledProcess
    if ($currentApp.Id -ne $appId -or $currentApp.StartTime.ToUniversalTime() -ne $appStart) { Finish-Test 'aborted' 'Installed process changed during acceptance.'; return }
    $focused = Test-OwnedFocus
    if ($focused) { $script:ownedFocusObserved = $true }
    $remaining = [Math]::Max(0, [Math]::Ceiling($TimeoutSeconds - ([DateTimeOffset]::UtcNow - $started).TotalSeconds))
    if (!$focused) { $status.Text = "Clique no campo para continuar. A aguardar: $remaining s. Nenhum input automatico." }
    else { $status.Text = "Foco confirmado no campo proprio. A aguardar ditado: $remaining s." }
    if ($script:textOutsideOwnedFocus) { Finish-Test 'aborted' 'Text changed without verified owned foreground/control.'; return }
    if ($box.Text.Length -gt 0) {
      $normalized = Get-Normalized $box.Text
      $script:keywords = @{
        reuniao = [bool]($normalized -match '\breuniao\b')
        quinta_feira = [bool]($normalized -match '\bquinta feira\b')
        dez_ou_10 = [bool]($normalized -match '\b(dez|10)\b')
        horas = [bool]($normalized -match '\bhoras\b')
      }
      $script:historyAfter = Get-HistoryMetadata
      $historyNew = $script:historyAfter.latest_timestamp_ms -gt $historyBefore.latest_timestamp_ms -and $script:historyAfter.latest_timestamp_ms -ge $started.ToUnixTimeMilliseconds()
      if ($script:keywords.reuniao -and $script:keywords.quinta_feira -and $script:keywords.dez_ou_10 -and $script:keywords.horas -and $historyNew -and
          ([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - $script:textObservedAtMs) -gt 1500) {
        Finish-Test 'passed' $null; return
      }
    }
    if ($remaining -eq 0) { Finish-Test 'not_accepted' 'Timeout waiting for recognizable test phrase in owned field plus a new history timestamp.' }
  } catch { Finish-Test 'aborted' 'Read-only safety check failed. No synthetic input was sent.' }
})
try { $form.ShowDialog() | Out-Null }
finally {
  $timer.Stop(); $timer.Dispose()
  if (!$script:finished) { $script:outcome = 'cancelled'; $script:failure = 'Owned test window closed by operator.' }
  $text = $box.Text
  $script:configAfter = Get-Hash $configPath
  $script:installedAfter = Get-Hash $installed
  if ($script:configAfter -ne $configBefore -or $script:installedAfter -ne $installedBefore) { $script:outcome = 'aborted'; $script:failure = 'Configuration or installed binary changed.' }
  $normalized = Get-Normalized $text
  # Check the raw string too: normalization alone removes non-Latin text,
  # which must never let alternative/private speech enter the receipt.
  $knownPattern = '^\s*A\s+reuni(?:' + [char]0xE3 + '|a)o\s+(?:' + [char]0xE9 + '|e)\s+na\s+quinta[ -]+feira\s+(?:' + [char]0xE0 + '|a)s\s+(?:dez|10)\s+horas[.!?]?\s*$'
  $exactKnownTest = [regex]::IsMatch($text, $knownPattern, [Text.RegularExpressions.RegexOptions]::IgnoreCase)
  # Alternative speech remains private: no raw text in console or receipt.
  $receipt = [ordered]@{
    schema = 1; outcome = $script:outcome; reason = $script:failure
    started_utc = $started.ToString('o'); finished_utc = [DateTimeOffset]::UtcNow.ToString('o')
    started_timestamp_ms = $started.ToUnixTimeMilliseconds(); field_changed_timestamp_ms = $script:textObservedAtMs
    installed_version = $version; installed_pid = $appId
    installed_sha256_before = $installedBefore; installed_sha256_after = $script:installedAfter
    config_sha256_before = $configBefore; config_sha256_after = $script:configAfter
    owned_foreground_pid_control_verified = $script:ownedFocusObserved
    text_arrived_outside_owned_focus = $script:textOutsideOwnedFocus
    text_length = $text.Length; text_sha256 = (Get-TextHash $text)
    exact_known_test_phrase = $exactKnownTest; test_field_text = $(if ($exactKnownTest) { $text } else { $null })
    expected_keywords = $script:keywords
    history_count_before = $historyBefore.count; history_count_after = $script:historyAfter.count
    history_appended = ($script:historyAfter.latest_timestamp_ms -gt $historyBefore.latest_timestamp_ms -and $script:historyAfter.latest_timestamp_ms -ge $started.ToUnixTimeMilliseconds())
    audio_captured_by_helper = $false; synthetic_input = $false; app_replaced = $false
    coverage = 'Human hotkey/speech using the already-running installed app; development candidate is not exercised.'
  }
  $destination = [IO.Path]::GetFullPath($ReceiptPath)
  [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($destination))
  [IO.File]::WriteAllText($destination, ($receipt | ConvertTo-Json -Depth 6), $utf8)
  $form.Dispose()
}
Write-Output ('outcome=' + $script:outcome + ' receipt=' + $destination)
if ($script:outcome -ne 'passed') { exit 2 }
