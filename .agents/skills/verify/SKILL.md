---
name: verify
description: Verify Lectus (chirp.exe) dictation changes live — launch with logs captured, drive hotkey+audio synthetically, read evidence from log + history.json
---

# Verifying Lectus live

## Launch with evidence capture

Logs go to stderr via env_logger (info level). The detached app loses them — relaunch redirected:

```powershell
Stop-Process -Name chirp -Force   # exe locked while running
$log = "<scratchpad>\chirp.log"
Start-Process C:\lt\release\chirp.exe -RedirectStandardError $log
```

Wait ~6 s: expect `pre-roll capture on input device`, whisper model load, `whisper warmup finished`.

## Drive a dictation without a human voice

1. **Read config first** — `%APPDATA%\ai.organic.lectus\config.json` for `hold_hotkey` / `toggle_hotkey` / `trigger_mode`. **The user changes these live; re-read before every run.**
2. The WH_KEYBOARD_LL hook does NOT filter injected input → `keybd_event` works for the hold chord.
3. Play the phrase via SAPI TTS through speakers while holding the chord — the SteelSeries Alias mic picks it up (verified). Expect mishearings on tiny model.
4. Evidence:
   - per-stage log lines: `pipeline: pre-roll contributed X ms` / `captured Xs of audio` / `transcription took X ms` / `injection took X ms` / `app profile matched`
   - transcript ground truth: last entry of `%APPDATA%\ai.organic.lectus\history.json`

## Injection target — use a controlled window

Do NOT use Notepad: Win11 `Start-Process notepad` returns a launcher PID that exits (AppActivate fails), and it restores the user's 39-tab session — injected text lands in a user document. Instead spawn an owned WinForms TextBox window (TopMost, dumps `.Text` to a file on a timer) and verify `GetForegroundWindow` title before triggering.

## Gotchas

- Config edits in Settings hot-reload the whisper model (log shows a fresh model load + warmup) — a mid-verification reload means the user is at the machine: STOP synthetic input.
- Notepad TabState bins only flush on close — useless for reading unsaved tabs mid-session.
- Timestamps in history.json are epoch ms; log times are UTC (local = GMT+1 in summer).
