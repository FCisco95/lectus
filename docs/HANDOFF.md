# Lectus — handoff

Updated: 2026-10-22. Branch feat/remote-dictation; dirty tree preserved. No commits, publication or installation.

## Active work (2026-11-13): setup wizard, free trial, vocabulary

Design approved by the owner on 2026-11-13. Nothing is built yet.
- Spec: docs/superpowers/specs/2026-11-13-setup-wizard-design.md
- Plan (11 tasks, start at Task 1): docs/superpowers/plans/2026-11-13-setup-wizard.md
- Execute with superpowers:executing-plans. The plan was written on Fable 5.2 so a Sonnet 5.5 session can run it. If a task fails twice, stop and report.
- Owner inputs still needed, none block the build: approve the legal documents (blocks fresh-install and installer acceptance only), approve three draft legal sentences, record four voice fixtures for the "Mycel" prompt benchmark.
- Found during research: the owner's config has language locked to `pt` and "Mycel" is often transcribed as "myself" (history: 1 correct, 3 "myself"). Parked next projects: English/Portuguese-only detection, new speech engines (Parakeet, Qwen3-ASR), learning from corrections.

The subscription AI state below is unchanged and still pending its live login test.

## TL;DR
**Subscription AI implemented, automated tests passed and reviewed. Real login/inference still pending.** Candidate SHA256 F41A66BEC7284339C6B9FF68DC40127B1C782135C688332B15CC7BEB47EDE60D; installation unchanged. Next action: coordinate readiness to temporarily close the current Lectus, open the candidate and complete official browser logins. Do not call the feature live-accepted before that test.

## Implemented
- ChatGPT: official OAuth PKCE loopback login, JWT/JWKS validation, native credential store, serialized refresh, account catalog and public Responses generation.
- Claude: unmodified official native CLI with its own Lectus account directory; safe-mode/restricted/no-tools plus managed-policy checks. Personal Pro/Max only; Team/Enterprise or detected managed policies blocked. Never imports global logins or uses API-key fallback.
- Both text providers wired into cleanup and explicit translation; connection/model/consent/enable controls stay separate. Profile enabling now works when global cleanup is off. Current permissions and account state are rechecked, including during preflight and reconnect/disconnect races.
- Dictionary/pt-PT instructions and cleanup-failure notice. Remote sender processes once; local HTTP API remains offline; history search remains substring matching, no semantic index added.

## Validation
- Native274pass/0fail/11ignored/2OS-mutation exclusions. Frontend77modules and release executable build passed; warnings remain.
- UI32 functional fixture scenarios +36 light/dark viewport captures passed. IPC mocked; not native acceptance.
- Official installed Claude2.1.308 probe passed with isolated authMethod none, matching configDirectory, absent local policy and accepted safe-mode argument syntax. No login or inference.
- Independent review findings (bare-mode/OAuth, disconnect authority, reconnect marker race) addressed; final scoped review found no remaining blocker.
-101 source files hashed and unchanged through final test/build. Portable receipt and review: docs/reviews/2026-10-22-subscription-ai-receipt.json and docs/reviews/2026-10-22-subscription-ai-review.md.

## Artefacts and installed state
Candidate: src-tauri/target/release/chirp.exe, 78926848 bytes, SHA256 F41A66BEC7284339C6B9FF68DC40127B1C782135C688332B15CC7BEB47EDE60D.
Installed binary freshly verified unchanged: CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406. Last observed installed PID3944; renew before action. No installed files/configuration/global account sessions changed. No actual subscriber credential created/imported. Native probe used temporary synthetic isolated auth-none config.

## Next actions
1. Await readiness/authorization for temporarily switching from running installed app to candidate without replacing installed files. Then finish official ChatGPT and Claude browser login; never request secrets in chat.
2. Run each card's synthetic Test and verify actual completed inference. Connection status/catalog alone are not proof.
3. Enable text permission/engine explicitly and test controlled dictation cleanup and translation preserving numbers, names and negation. Record native evidence separately.
4. Only after acceptance decide installation with reviewed exact hashes/backup helper. macOS remains uncompiled/unexecuted; hardware/NSIS/legal release gates unchanged.

Older USB/suspend/installer/legal tasks remain in docs/handoffs/2026-10-21-before-subscription-ai.md. Do not restart a USB observer without explicit human readiness or repeat valid old checks routinely.

## Evidence correction
The old fixed-output guided UI script overwrote October21 raw screenshots/results during an October22 run. EVIDENCE-NOTE.md at that old directory explains scope; original narrative remains historical, raw old evidence was not recovered. New runs use UI_SMOKE_OUTPUT and dated folders. Historical native receipts unaffected.

## Repo prune (2026-11-11)
Deleted 31 tracked files: 20 of 23 dated handoffs, LECTUS-NEXT-PROMPT, phase2.5 plan, deploy-pipeline and app-shell specs, remote-dictation plan, docs/mockups, release-notes-0.7.1. Restore any file: `git show pre-cleanup-2026-11:<path>`. Open item carried from the deleted remote-dictation plan: real Windows-to-Mac insertion test not yet done. Kept pending a decision: two volume-bug handoffs, home-window and organic-token-gate specs, .claude/workflows/deep-research.js, mobile icons. Untracked artefacts (.context, scripts/diagnostics, docs/reviews) not touched.

## Suggested skills
verify and computer-use for authorized native acceptance; verification-before-completion; handoff; official OpenAI docs for protocol changes.

## Generated artifacts this session
Design/plan, subscriptions source and UI, local executable, source baseline and detailed logs/reports under .context/subscription-ai, portable docs/reviews/2026-10-22-subscription-ai* and three dated UI receipt directories. No accounts/services/real credentials created, no install/publish/commit. Shared task-observer observation0018 extended with authentication-mode smoke limitation; no skill changed.

## Next-session prompt
```text
Continue Lectus subscription AI from docs/HANDOFF.md. CandidateF41A66B... is built/reviewed with274 native tests passing; installed app unchanged. No human login or authenticated inference accepted yet.
Files: docs/reviews/2026-10-22-subscription-ai.md, docs/reviews/2026-10-22-subscription-ai-receipt.json, docs/superpowers/plans/2026-10-22-subscription-ai.md, src-tauri/src/subscriptions/mod.rs, src/components/settings/AIConnections.tsx.
Model: gpt-6.1-sol high — native acceptance and provider diagnosis.
Skills: verify, computer-use, verification-before-completion, handoff.
Coordinate readiness before closing the installed app and opening the exact candidate temporarily; finish official logins and test synthetic text on each provider. Preserve installed files and all unrelated dirty work.
```
