# Lectus — macOS app updated to v0.6.0

## TL;DR

The installed macOS Lectus app now runs the unreleased app-shell UX/UI build at
`fix/macos-accessibility-loop` commit `03588a1`, based on
`origin/feat/app-shell-canvas` commit `c2ea774`. It was built locally and its
redesigned Home window was verified. The branch still reports v0.6.0 because it
was never version-bumped. The repeated Accessibility prompt was fixed in code;
the current local binary must still be added once in macOS Accessibility after
removing its stale prior entry.

## What to do next

Continue with the planned local transcription API specification and implementation.
Do not change capture, injection, or shortcut handling without an explicit request.

## Suggested skills

- `handoff-memory`
- `verify` for live app checks

## Generated artifacts this session

| What | Where it lives | Notes |
|---|---|---|
| macOS rollback app bundles | `/Applications/Lectus.app.v0.6.0.release.backup`, `/Applications/Lectus.app.v0.5.0.backup` | Local-only fallbacks after the app-shell build launched. |

## Next-session prompt

```
Lectus is installed on macOS from the unreleased app-shell UX/UI branch `origin/feat/app-shell-canvas` at commit `c2ea774`; its manifest still reports v0.6.0. Local signed v0.6.0 and old v0.5.0 rollback bundles remain outside the repo. The next product slice is the local transcription API; do not touch capture, injection, or shortcut handling.

Files: docs/HANDOFF.md, docs/superpowers/specs/2026-09-17-organic-token-gate.md, src-tauri/src/updates.rs
Model: Codex Sonnet 5 — focused Rust/Tauri product planning and implementation.
Skills: handoff-memory, verify

Spec the local `POST /v1/transcribe` API before coding: per-install token, explicit Organic origin allowlist, off by default with visible activity state, and no remote microphone control in v1.
```
