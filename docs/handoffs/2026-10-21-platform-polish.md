# Lectus — platform polish checkpoint

Updated 2026-10-21. Branch: `feat/remote-dictation`. Changes are local and uncommitted.

## Current state

UI polish, model selection/connectivity controls and reliability fixes are implemented and verified locally. Preserve the current Organic shell. The Windows executable built successfully but was NOT installed or published. Next: review the build/diff, arrange a real native smoke test, then package/install through the existing release workflow when authorized.

The prior installed 0.7.1 volume patch remains the installed version. This session did not restart Lectus, replace its executable, change OS audio settings or publish a release. The intermittent Windows volume issue is still unresolved; history is in `docs/handoffs/2026-10-12-bluetooth-volume-investigation.md`.

## Changes and evidence

Read `docs/reviews/2026-10-21-platform-review.md` for delivery scope, exact verification and limits. Read `docs/plans/2026-10-21-platform-polish.md` for the design/model decisions and reconciliation of older plans.

- Keyboard controls/labels, contrast, responsive settings, truthful readiness/destination, history confirmation/errors and modal focus restoration.
- Configurable cloud speech and cleanup models plus a metadata connection check. Cleanup default is GPT-OSS 120B; 20B is selectable. Old hardcoded Llama 3.3 is deprecated for Groq free/developer accounts. Local catalogue includes full Large v3 and quantized Turbo; Base stays the new-install default.
- Serialized saves and model activation, per-file downloads, retained errors/progress, safer persistence and stale async response guards.
- 167 native tests passed, 0 failed, 5 optional tests ignored; clipboard replacement and real mute roundtrip excluded. Frontend and Windows release build passed.
- 10 browser fixture scenarios + 16 captures passed. These mock IPC and do not establish native integration.
- Real offline RTX 3080 benchmark passed: 7.6s Portuguese fixture warm with VAD, Base 148ms and Turbo 384ms best-of-three. Both recognized quinta-feira; Tiny did not. Three short fixtures are not a comprehensive quality ranking.

Build: `src-tauri/target/release/chirp.exe`, SHA256 `C07BE8D24ADC9FDC2C79EEC6A2AB4417CBD11AF8C07375FDE5D0B66512C159E5`. Version remains 0.7.1, development build only. Build/test helper `scripts/verify-native.cmd` never deploys; `--build` builds, `--refresh-permissions` refreshes stale moved-checkout Tauri artifacts. Do not use `build-release.cmd` casually: it invokes local deployment.

## Outstanding

1. Native smoke of the updated binary, including settings/model operations and controlled dictation; package/install separately. Do not replace/restart a running user app unexpectedly.
2. Real Windows-to-Mac pairing/check/insertion. Prior transport implementation and released 0.7.1 remain documented in `docs/REMOTE-DICTATION.md` and `docs/handoffs/2026-10-11-remote-dictation.md`.
3. Account-level cloud tests: installed config had cloud off, cleanup off and no API key. Do not ask for secrets in chat. User can configure the provider in General; metadata listing is not inference authorization.
4. Local transcription API: `docs/superpowers/specs/2026-09-17-local-transcription-api.md` remains specified, unimplemented. Do not expose a partial server; keep its full authentication/origin/size/local-only requirements.
5. Intermittent Windows/Bluetooth volume issue remains open; prepared registry test was not applied. New full Large v3/quantized Turbo choices also remain unbenchmarked locally.

## Suggested skills

`handoff-memory`, `systematic-debugging` when investigating a failure, `verify` for a controlled native test, Impeccable for UI work, `verification-before-completion`, `handoff`. Review agents were explicitly requested and used. Impeccable was read upstream; its launcher was not installed.

## Generated artifacts this session

| Artifact | Location | Notes |
|---|---|---|
| Plan/review | `docs/plans/2026-10-21-platform-polish.md`, `docs/reviews/2026-10-21-platform-review.md` | Scope, decisions, validation and limitations |
| UI harness/evidence | `scripts/ui-smoke.mjs`, `docs/reviews/2026-10-21-ui-verification.md`, `docs/reviews/2026-10-21-ui-smoke/` | Synthetic fixtures; 16 PNGs + JSON receipt |
| Real model benchmark | `docs/benchmarks/2026-10-21-windows-models.md` | Portable timings; local ignored raw log alongside |
| Native helper/build | `scripts/verify-native.cmd`, `src-tauri/target/release/chirp.exe` | Executable untracked build output; not installed |
| Handoff | `docs/HANDOFF.md`, `docs/handoffs/2026-10-21-platform-polish.md` | No new credentials or external resources |

## Next-session prompt

```text
Lectus has a verified local UI/model/reliability refinement on feat/remote-dictation, uncommitted and not installed/published. Read docs/HANDOFF.md and docs/reviews/2026-10-21-platform-review.md. Preserve the installed volume patch and existing shell.
Files: docs/plans/2026-10-21-platform-polish.md, scripts/verify-native.cmd, scripts/ui-smoke.mjs, docs/REMOTE-DICTATION.md, docs/superpowers/specs/2026-09-17-local-transcription-api.md.
Use the current coding model with high reasoning for integration review; no model switch is required.
Skills: handoff-memory, verify, verification-before-completion, handoff.
Review the pending diff and carry out a controlled native smoke test before preparing a new package. Keep cloud account access, real Mac insertion, unresolved volume drops and the unimplemented local API explicitly separate from the passing local evidence.
```
