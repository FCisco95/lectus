# Lectus - persistence / daily-launch (2026-09-17)

Canonical state: `docs/HANDOFF.md`.

User: app feels sloppy each morning; words and model do not seem stored.

Evidence on disk before any code change:

- `config.json` already had `ggml-large-v3-turbo.bin`, `Mycel`/`Claude`,
  `onboarding_completed: true`
- `history.json` at cap (100)
- Autostart pointed at `%LocalAppData%\Lectus\chirp.exe`

Root causes:

1. `AppState::new()` started as `Config::default()`. Settings webview could
   `get_config` before `setup()` loaded disk → onboarding flash, empty vocab,
   wrong Active model. Completing that onboarding saved the stale snapshot.
2. Dictionary words only fed Whisper `initial_prompt`. No post-ASR rewrite,
   so custom spellings did not appear — felt like they were never saved.
3. `LocalWhisper::new` of large-v3-turbo ran on the setup thread, blocking
   tray/hotkey until the 1.6 GB model was in VRAM.

Fixes deployed to the live install 2026-09-17 09:19. 76 lib tests green.
Live dictation of dictionary spellings not agent-verified (user at machine).
