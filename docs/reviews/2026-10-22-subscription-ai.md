# Lectus — subscription AI integration

Status: source implemented, automated verification passed, independent review has no remaining blockers. Candidate built; not installed or accepted live.

## Scope
Local desktop ChatGPT sign-in and official Claude Code sign-in, with separate connections, shared per-provider text model and explicit transcript permission. Cleanup and translation select providers independently. Speech recognition remains local or explicitly configured API; no subscription-to-paid-API fallback.

All native dictation entrypoints converge in do_pipeline. Dictionary spelling/replacements run before cleanup, per-app profiles select effective cleanup behavior, and translation remains explicit. Remote sender processes once; receiver inserts delivered text without additional AI. History retains existing recovery behavior. Local HTTP API stays offline-only; history search remains substring search, not semantic indexing.

## Verification boundaries
See dated subscription and guided UI receipts in this directory: IPC is mocked and is not native acceptance. Final native suite:274 passed,0 failed,11 ignored,2 OS-mutation tests excluded. Frontend77modules and native release build passed; compiler warnings remain. UI32 functional scenarios and36 screenshots passed. The101 recorded source files were unchanged across final verification; see the JSON receipt. Official CLI syntax/auth-isolation probe is not an authenticated inference test. Human browser login and a completed synthetic generation are required for each provider. No installed executable, user configuration or global login has been replaced by this work.

## Review findings
- Effective-profile cleanup was blocked by a global enable check; corrected along with current consent/model checks.
- Preference changes could hide the completion of disconnect; separate test and lifecycle revisions fix this.
- Privacy revocation during provider preflight needed a captured inference epoch; configuration updates now invalidate pending text dispatch without cancelling account login.
- Final review: bare-mode blocks Claude OAuth; new requests could capture a fresh epoch during disconnect before credential deletion. Both fixed; reconnect marker recovery also serialized with activation. Scoped re-review confirmed no remaining blockers.

## Evidence provenance correction
The existing guided UI harness used a fixed October21 output directory. An October22 rerun overwrote its raw screenshots/results before a useful old backup was made. That directory has EVIDENCE-NOTE.md; it no longer proves the October21 candidate. The prior narrative remains historical. Current verification is saved separately in 2026-10-22-subscription-guided-smoke; UI_SMOKE_OUTPUT supports explicit destinations. Native historical receipts were not touched.

## Sources
- https://developers.openai.com/siwc/token-sharing-open-source/sign-in
- https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference
- https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations
- https://code.claude.com/docs/en/cli-reference
- https://code.claude.com/docs/en/headless
- https://code.claude.com/docs/en/legal-and-compliance

## Plan
../superpowers/plans/2026-10-22-subscription-ai.md

## Ready candidate

`src-tauri/target/release/chirp.exe`, 78926848 bytes; SHA256 `F41A66BEC7284339C6B9FF68DC40127B1C782135C688332B15CC7BEB47EDE60D`. Installed binary unchanged (`CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406`). No local installation or live authentication accepted.

Claude currently supports personal Pro/Max accounts only after checking that managed policy sources are absent. Team/Enterprise or detected managed policy are explicitly blocked because OAuth-compatible CLI modes cannot override managed hooks. macOS code is not compiled/executed in this Windows receipt. ChatGPT uses account-specific eligible models.

## Next acceptance step

1. Coordinate a safe moment to close the running installed Lectus and open this candidate temporarily, preserving the installed files.
2. In Settings → AI cleanup, finish ChatGPT login in the system browser, choose an available model and run Test ChatGPT.
3. Finish the official Claude login from the Claude card and run Test Claude. No passwords/tokens in chat.
4. Explicitly allow transcript processing and choose cleanup/translation engines; verify a controlled synthetic sentence, numbers and negation.
5. Record real results separately from these fixtures; only then decide whether to install the candidate.
