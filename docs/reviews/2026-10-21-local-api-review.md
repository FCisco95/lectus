# Local transcription API — implementation receipt

Date: 2026-10-21. Local source/build only; installed-app restart decision is pending.

## Delivered

The remaining local API specification is implemented with a Settings tab, explicit save, masked token/reveal/copy/rotation, five-call metadata list, pill indicator and tray status. It is off by default and accepts uploaded WAV only, on IPv4 loopback. It reuses local Whisper and dictionary rules; it does not capture, inject, write history, invoke cloud models or spawn CLI cleanup.

Authentication, exact Host/origin/CORS checks, membership, one-job concurrency, 30 authenticated requests/minute, bounded socket workers/deadlines and WAV shape/size/duration limits are enforced. Config changes and token rotation invalidate uploads before inference admission. An admitted inference may finish after disable; UI and tray show that until completion.

Implementation decisions and ownership: [plan](../plans/2026-10-21-local-api-implementation.md). Setup and portable browser integration: [local API guide](../LOCAL-API.md), [client example](../../examples/lectus-client.mjs).

## Verified

| Check | Result |
|---|---|
| Frontend production build | Passed, 70 modules |
| Native release suite | 184 passed, 0 failed, 6 optional tests ignored, 2 deliberately excluded clipboard/audio mutation tests |
| Real HTTP → local Whisper | Passed; 7,628ms Portuguese WAV processed in 116ms, language `pt`, “quinta-feira” recognized; unauthorized health refused |
| API browser fixture | 7 behavioral scenarios + 8 light/dark 1100×800 / 720×540 captures passed; final receipt 15:35:12Z |
| Windows release executable | Built successfully in 57.36s |
| Independent backend review | No remaining blocker reported after fixes |
| Install preparation | Dry-run only: verified source/destination, current PID and hashes; installer backs up binaries and verifies every copied file |

Real inference test uses temporary API settings/token, an explicitly allowed test backend, the downloaded Base model, and repository fixture audio. It proves HTTP parsing, authentication, WAV decoding and actual model inference together. Membership refusal is covered by protocol tests; this test does not claim the running user's membership or native webview integration. No persistent user API token was generated.

Protocol tests cover wrong/malformed auth, exact origins, Host rebinding, preflight, unsupported headers/media/audio, oversized input, rate limits, metadata cap, busy/membership/readiness, concurrent rejection, pending-upload revocation, failed persistence/bind, reopen, worker limits and slow-header deadlines.

Build: `src-tauri/target/release/chirp.exe`, SHA256 `E35FE4B8865037CECC1DB06F372B9F3A53C9EDACE91EFFD741BC4FE0DB1068B1`. Version remains 0.7.1, local development candidate. The installed executable still matches the prior volume-fix hash `E83CD527774730187FBAD1428DD14B16F18E1941A83CEEEFA7151B4A3353438C` and was observed running as PID 24836. PIDs are historical, re-query before acting.

## Review fixes

- Fixed a temporary-mutex lifetime deadlock in a preflight test before executing the suite.
- Explicitly reject wildcard origins on the backend as well as in the form.
- Make generation validation and inference admission atomic; release locks before callbacks or model work.
- Distinguish upload/dictation busy state from actual API inference, avoiding a misleading pill pulse.
- Keep “Finishing current API request” visible after disable until admitted inference completes.

## Remaining delivery checks

The running app has not been stopped or replaced. A restart/install approval question is pending because replacement would interrupt an active dictation. `scripts/install-local.ps1` defaults to a dry run; actual installation requires `-Apply -Restart` and should pin the reviewed hash with `-ExpectedSHA256`.

Real installed native UI checks, browser loopback permission behavior and Organic integration remain unverified. No other repository, public release, account settings, OS audio setting or user model choice was changed. The prior Windows volume issue and real Mac insertion checks remain open. Existing compiler warnings remain.
