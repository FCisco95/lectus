# Bluetooth volume investigation

## 2026-10-12: persistent volume regression investigation

User reports intermittent drops to Windows volume 1 after dictation and manual
slider increases bouncing back down. Installed chirp.exe still matches the local
volume-fix SHA256; process 41560 remains running. The earlier patch did not fully
resolve the reported symptom. No new Lectus code edits or OS setting changes today.

Read-only registry check: `HKLM/SYSTEM/CurrentControlSet/Control/Bluetooth/Audio/AVRCP/CT`,
`DisableAbsoluteVolume` is DWORD 0 (enabled). UserDuckingPreference was not explicitly
present in the inspected HKCU Multimedia/Audio key. Do not infer a configured value
from absence. Active output was previously confirmed Logi Z407 via Bluetooth.

Leading hypothesis to test: Windows/speaker absolute-volume synchronization.
It remains unconfirmed; no reproduction with Lectus fully exited has been reported.
Archived Microsoft guidance documents incompatible Bluetooth devices and a reversible
DisableAbsoluteVolume=1 workaround (old Windows 10 guidance, not proof for this Z407).
Source: https://learn.microsoft.com/en-us/answers/questions/2855800/windows-10-version-1803-april-2018-update-broke-bl?page=26
A first-person Z407 report describes increasing a phone volume slider causing volume
to decrease; anecdotal corroboration only (search excerpt retrieved, full page fetch failed):
https://www.reddit.com/r/logitech/comments/sr9p7s
Logitech troubleshooting also suggests testing one Bluetooth source at a time:
https://support.logi.com/hc/en-us/articles/360053049953-No-audio-from-the-Z407-speakers

## Prepared diagnostic artifacts (NOT APPLIED)

In the Input Leap workspace: `setup-downloads/bluetooth-volume-test/` contains
`disable-volume-sync.reg` (DWORD 1), `restore-volume-sync.reg` (original DWORD 0),
and `original-setting.json`. The registry was read again and remained 0.
The test affects Bluetooth volume synchronization globally and may require a Windows
restart. Do not restart Windows without explicit authorization. Ask before applying
this broader OS change; the user's current request was investigation/search.
If authorized, apply only this setting, verify it, arrange restart, test normal
manual adjustments and several dictations, and restore 0 if it does not help.

## Suggested skills

`systematic-debugging`, `verification-before-completion`, `handoff`.

## Next-session prompt

```
The local Lectus volume patch is installed but intermittent volume drops persist.
Read docs/HANDOFF.md and src-tauri/src/playback.rs. Bluetooth Absolute Volume is
currently enabled; a reversible test is prepared but has NOT been applied.
Model: Codex Sonnet 5 (high) — focused diagnosis per user routing.
Skills: handoff-memory, systematic-debugging, verification-before-completion.
Resolve whether the user authorizes the prepared Bluetooth setting test; do not
claim the Z407 is faulty or reboot the PC without permission.
```
