# Lectus - persistence / stale-install audit

Snapshot of 2026-09-15. Canonical state: `docs/HANDOFF.md`.

User reported: words not stored while talking, selected model lost after power-off,
sometimes the previous app version opens.

Root causes found on this PC:

- HKCU Run `Lectus` = `C:\lt\release\chirp.exe` (0.4.0, 2026-08-26) while
  shortcuts already launched `%LocalAppData%\Lectus\chirp.exe` (0.5.0).
- `save_config` wrote the stale Settings snapshot (including old `model_name`)
  to disk, then only restored `model_path` in memory.
- History append ran only after `inject_text` succeeded.

Fixes in source + local deploy to the install dir. Autostart Run key rewritten
immediately. 58 lib tests green. Vulkan warmup 173 ms on RTX 3080 after relaunch.
