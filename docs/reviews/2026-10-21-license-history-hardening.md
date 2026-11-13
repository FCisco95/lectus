# Lectus — model agreements, history and local installation hardening

2026-10-21. Development work on the dirty `feat/remote-dictation` tree. No installation, publication, real model agreement acceptance, cloud call, microphone or clipboard mutation occurred in this continuation.

## Changes and boundaries

- Gemma cleanup has its own explicit optional-model agreement. Bundled readable Terms and incorporated use restrictions retain source HTML, extraction provenance and content hashes. Local receipts bind model ID, document version, fingerprint and time. Status reads never accept. Download, startup warmup, cached-model reuse and local cleanup require current acceptance. Revocation leaves downloaded files/history intact and does not enable cloud processing. Qwen translation remains independent.
- The model agreement panel provides keyboard-readable documents, an unchecked choice, explicit accept/revoke, retry/reload and errors. A review found that notifications could overtake mutations across windows; command mutation and notification now share a serialization guard, covered by a channel-coordinated regression. Already admitted model operations may finish after revocation; later admissions are refused.
- History append and clear serialize the full read/modify/write operation. Corrupt/unreadable history fails closed rather than being overwritten as an empty file. Atomic temporary writes are synced. Isolated tests cover concurrent operations, preservation on failures and translated-source recovery.
- The local installation helper previews the complete runtime/resource manifest. Applying requires the reviewed `ExpectedManifestSHA256`, freezes candidate files, verifies backup/replacement hashes and rolls back exact changed files. It refuses traversal/reparse paths and removes only introduced files and empty directories during rollback. Launch failure is reported separately from successful file installation. No actual install/repair/uninstall acceptance is claimed.
- Native and packaging legal readiness now share 18 fixture cases. Drafts, missing publisher identity or unreviewed obligations continue to block public packaging. Support/privacy contact remains `bakehouselabs@proton.me`; no private legal name was copied into public files and no company was invented.

## Third-party evidence

Read [NOTICE-RECONCILIATION.md](../legal/NOTICE-RECONCILIATION.md). All 130 original source-package gaps are classified. All 11 Windows runtime candidates among those gaps now have full exact-source license texts. This is not a finished installer SBOM or a legal approval. The macOS declaration/SDK caveats, three metadata-only MIT cases, actual shipped artifacts and all resulting obligations still need release review. An offline collector repeat retained inventory hash `e817e074189f4e89dc1d55a456a5f8e8348a4cf9d7f64023cf4860b2240419c2`.

## Verification

Raw `.log` files named below are local, gitignored diagnostics. The durable results and decisive failure details are recorded in this report; do not assume those logs will exist on another machine.

- Frontend production build passed, 74 modules. Model-panel fixture evidence is maintained in [the UI report](2026-10-21-model-license-verification.md); those are synthetic IPC fixtures, not native UI acceptance.
- Node legal-readiness tests: 18 passed. Actual `node scripts/check-legal.mjs --release` returned `ready:false` with the expected five unresolved-input groups.
- Local installer tests: six isolated checks passed; no real installation or process was changed. The real read-only plan enumerated 1,308 files and returned `Applied:false`.
- Actual Qwen test passed all ten short translation requests: Portuguese and Chinese to English, English to Simplified/Traditional Chinese and Portuguese, two passes. First call 2,214 ms; warm second pass 203–275 ms on this RTX3080. See `docs/benchmarks/2026-10-21-model-license-independence.log`. This is smoke evidence, not a language-quality ranking. No real Gemma agreement was accepted for it.
- Native suite initially passed 217 tests. After adding the notification-order regression, it passed that regression but exposed two API response assertions: 216 passed, two failed, seven ignored and two deliberate OS-mutation exclusions. The first failing log is preserved as `docs/reviews/2026-10-21-hardening-native.log`. The corrected full suite subsequently passed 220 tests, zero failures, seven optional ignored and two deliberate clipboard/audio-mutation exclusions; final-source confirmation is recorded with the candidate below.
- Native development release build passed. Build warnings remain; no blanket warning-free claim. A CMD/Visual Studio hop retained a PowerShell 7 module path when launching Windows PowerShell, breaking `Get-FileHash` during backend staging. The helper now explicitly imports the current host's built-in Utility module; actual translation reran successfully.

## API rejection-response correction

Focused reproduction failed on round two with this response, truncated before the header terminator and declared 81-byte body:

```text
HTTP/1.1 403 Error\r\nContent-Type: application/json\r\nContent-Length: 81\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Ty
```

The membership decision was correct. Dropping the socket while upload bytes remained unread caused a TCP reset that could erase the response in the client's receive buffer. The server now completes the response, shuts down its write half, then discards pending input for at most 250 ms and 8 MiB before closing. The final read is sliced to the remaining byte budget. No rejected audio is decoded or admitted, and no second request is accepted. This follows the staged-close rationale in [RFC 9112 section 9.6](https://www.rfc-editor.org/rfc/rfc9112.html#section-9.6).

Strict test assertions now require a header terminator and the full declared body. New regressions exercise a nonclosing peer and 20 uploads arriving during rejection. The focused API suite passed 19 tests, followed by 20 consecutive passing rounds (380 test executions, including 400 late-upload rejection requests). These repeated rounds preceded the final buffer-slice tightening; the full suite verifies that final adjustment. No test assertion was weakened or production deadline increased to hide the failure.

## Final development candidate

Final-source native suite: **220 passed, zero failed, seven optional ignored, two deliberate clipboard/audio-mutation exclusions**, 5.54 seconds. The subsequent native release build succeeded (16 existing warnings). Frontend production build and final UI fixture fingerprint above remained unchanged. `git diff --check` passed.

| Artifact | SHA256 / result |
|---|---|
| Development `src-tauri/target/release/chirp.exe` | `E2872491D364731315A50CEC440F0C1EB4DF07F9A4EF2FF64C6C7B1634FE2E8A` |
| Read-only local installation manifest | `9EF6F6A65E0CB7F982B35E59BB353E6B0583213A9327DAFAD9CC965F9E6E0FFE` |
| Manifest file count / applied | 1,308 / false |
| Existing installed executable, rechecked unchanged | `CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406` |

Version remains 0.7.1 development. These hashes identify a local candidate, not a signed or approved installer. Re-preview the full manifest if any candidate binary or resource changes. The installer helper was not applied.

## Remaining release work

Publisher identity and final Terms/Privacy/model/third-party obligations remain unresolved. The earlier installed version is unchanged. The older NSIS artifact must not be presented as containing this work. Real fresh-install/upgrade/uninstall, native microphone/clipboard, live provider accounts and Windows-to-Mac acceptance remain separate checks. No signing certificate or legal structure was purchased or created.
