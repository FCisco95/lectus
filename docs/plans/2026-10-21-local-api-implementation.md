# Local transcription API implementation

Continuation of the existing September specification, requested 2026-10-21 after the platform refinement. Keep all prior dirty work. No installed-app replacement, OS audio change or release publication is part of implementation.

## Settled contracts

- Default off, bind only `127.0.0.1`, default port 8756, explicit save in Settings → API. Empty origin allowlist blocks browsers; no production origins inferred.
- Upload-only WAV, mono 16kHz, at most 8 MiB / 120 seconds. No recording, insertion, history access, cloud requests or CLI invocation.
- Bearer auth on health and transcription. Browser preflight is the intentional token-free exception, requiring exact allowed origin/Host/method/headers.
- Membership checked per request and immediately before inference, without raising windows. One API inference at a time; reject while dictation or model loading is active; API jobs cannot interrupt inference already underway.
- Local dictionary/rules apply. Optional cleanup uses only the enabled local cleanup engine. This resolves the old spec's conflicting “no network” versus “Claude CLI cleanup” paragraphs in favor of the stronger local-only boundary.
- Settings and token share `api.json`, separate from general config autosave, so a stale general form cannot clobber API state. Token generated on first enable, masked in UI, excluded from status/events/logs.
- Bounded socket workers, bounded headers, absolute upload deadline and generation checks prevent stale uploads after disable/rotation. Raw `std::net` HTTP handling is used to enforce these bounds without new dependencies.
- Last five calls store only time/origin/audio duration/result length/error code. Activity uses `api-activity`, a distinct pill indicator and tray tooltip.

## Ownership

1. Backend worker: `src-tauri/src/api/**`, protocol/lifecycle/audio/token tests.
2. UI worker: `ApiPanel.tsx`, Settings modal integration, pill and API styles.
3. Primary: `api_adapter.rs`, `lib.rs` integration, real-inference HTTP test, client example/docs, native build/review/handoff.
4. Verification worker: browser fixtures and API screenshots. No real user state or secrets in fixtures.

## Acceptance

Run protocol tests for authentication, exact origins, rebinding Host, preflight, media/size/duration, no inference before authorization, membership/busy, rate, disable/rotation and persistence failures. Run the service against a real local model and repository WAV in an isolated test configuration. Check API UI default-off, explicit save, error recovery, token reveal/copy/rotation and light/dark compact layouts. Build frontend/native; obtain independent review before completion.

Actual browser permission prompts, installed-app native interactions and cross-repo Organic integration remain separately identified live checks. The portable client example documents conversion and request handling without modifying another repository.
