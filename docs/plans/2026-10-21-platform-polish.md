# Lectus platform polish and reliability

User requested continuation of the plans, selectable model updates, connectivity checks, code simplification and an Impeccable-guided UI revamp with review agents.

## Direction

Preserve the existing Organic identity and native app shell. Prioritize trustworthy state, accessible controls, recovery and model lifecycle correctness over ornamental changes. Impeccable 4.7.0 was read from its upstream repository; no local launcher was installed. Its Operate/polish guidance applies to this existing app.

## Implementation and ownership

1. UI lane: keyboard controls, truthful Home readiness/mode/destination, history deletion confirmation, readable light/dark tokens and narrow-window reflow.
2. Model lifecycle lane: coordinate downloads/activation, preserve errors/progress across settings remount, display actual loaded model, persist settings before committing memory.
3. Integration lane: configurable speech/cleanup model IDs, provider metadata check, serialized autosave and atomic config writes, mock HTTP regression tests.
4. Verification: frontend build, browser fixtures explicitly distinguished from Tauri runtime, native suite with clipboard/audio-changing tests excluded, independent review and portable handoff.

## Model decisions and evidence

- Keep the local Base default: the existing M4 benchmark supports it; a newer/larger model is not automatically best for each device.
- Expose full Whisper Large v3 and quantized Large v3 Turbo alongside existing models. Downloading is explicit; catalogue availability is not a local speed/accuracy benchmark.
- Groq speech: `whisper-large-v3-turbo` and `whisper-large-v3` remain production offerings.
- Cleanup: replace the old hardcoded Llama 3.3 ID with configurable `openai/gpt-oss-120b`, with 20B offered for speed. Groq deprecated Llama 3.3 for free/developer accounts on 2026-08-16; enterprise exceptions exist. GPT-OSS uses low reasoning and excludes reasoning from returned content on the Groq endpoint.
- A `/models` response confirms metadata access only. It cannot prove inference permission, dictation accuracy or remote insertion.

Sources checked 2026-10-21: [Groq models](https://console.groq.com/docs/models), [deprecations](https://console.groq.com/docs/deprecations), [reasoning](https://console.groq.com/docs/reasoning), [Whisper model files](https://huggingface.co/ggerganov/whisper.cpp/tree/main), [Impeccable](https://github.com/pbakaus/impeccable/blob/main/.agents/skills/impeccable/SKILL.md).

## Reconciliation of older plans

- Phase 2.5's unchecked implementation steps are historical; the hook, hold-to-talk, native injection, model picker and current shell already exist. Do not replay its branch setup or old dependency instructions.
- Remote dictation is implemented with transport tests. Real Windows-to-Mac insertion remains unverified on the current devices.
- Windows intermittent volume drops remain unresolved; do not change Bluetooth registry settings or claim this UI work fixes them.
- `docs/superpowers/specs/2026-09-17-local-transcription-api.md` is a separate, specified but unimplemented feature. Its loopback authentication, origin checks, bounds, lifecycle, UI visibility and local-only behavior require a complete implementation stage; this polish pass does not expose a partial server.
- Capture, injection and shortcut behavior are outside this refinement's changes. No app replacement or release publication is included.

## Acceptance

Controls usable by keyboard; errors persist visibly; settings survive serialized saves and old config migration; model requests contain the chosen IDs; duplicate model operations cannot overwrite the same partial file or replace the active engine out of order; browser layout checked in both themes at 720×540 and 1100×800. Native/live-device limits must be reported separately.
