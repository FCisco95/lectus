# Lectus - mute while dictating

Snapshot of 2026-09-15. Canonical state: `docs/HANDOFF.md`.

Feature: duck system playback for the hold so music/YouTube/games go quiet,
then always restore.

Windows implementation mutes the default render endpoint
(`IAudioEndpointVolume`). Devices that reject mute fall back to volume 0 and
restore the previous scalar. Already-muted devices are left muted.

Restore paths: hold-stop, pipeline error, idle, process exit, Drop.
Setting persisted as `mute_while_dictating` (default on). Capture/VAD/inject
untouched.

68 lib tests green, including a live COM mute roundtrip. Deployed to
`%LocalAppData%\Lectus\chirp.exe`. Synthetic dictation verify did not complete
(desktop in use).
