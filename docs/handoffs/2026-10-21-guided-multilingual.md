# Lectus — guided multilingual development checkpoint

Updated 2026-10-21. Branch `feat/remote-dictation`; all work remains local, uncommitted and unpublished. Preserve the whole dirty tree and prior API/platform work.

## Current state

The user installed the earlier local API/platform installer themselves. Installed hash was confirmed as `CD41E53A3A337B7770C99D9EA2E779546B679C39AD5EBA5C441E0DCCA117C406` (PID25604 at that observation). The new translation/help/onboarding/legal work below is NOT installed. No new public installer was produced; the actual Tauri bundler was verified to refuse drafts before packaging.

The user's current goal is an easy-to-understand app with local/cloud translation choices, contextual why/how/privacy help, a professional installer/setup flow, and legally appropriate terms handling. They explicitly approved delaying microphone and dictation until setup completion.

## Private identity boundary

The user confirmed Portugal and is considering independent-developer publication, but is concerned about doxxing. Do not copy the private name from conversation into public files, invent a company, request home address/tax number in chat, create accounts or contact third parties. Public publisher/contact arrangements remain unresolved; manifest fields are deliberately blank. A dedicated public support/privacy email is appropriate, but does not by itself settle applicable Portuguese identification obligations. Terms acceptance cannot guarantee immunity or legal compliance.

## Implemented

- Output language separate from speech recognition, default Same as spoken. Local Qwen3 4B and explicitly configured cloud translation; Simplified/Traditional Chinese and Cantonese choices. No cross-provider fallback. This is a text translation model, NOT a new Qwen3-ASR speech engine.
- Original-source recovery in translated history; failure inserts nothing and accurately reports whether history was saved. UI Copy original and persistent recovery errors.
- Help hub and contextual HelpTip across settings; guided setup with progress, review, explicit mic permission action, saved hotkey and real error states.
- Backend-owned setup completion; microphone, local/API dictation and remote insertion enforce setup/legal state. Generic config cannot mark setup complete.
- Separate configured-provider and Claude CLI acknowledgements; URL changes reset provider permission and normalized config returns to UI. Admission rechecks prevent queued work using revoked/different provider authority. HTTPS required except loopback; redirects disabled.
- Local legal receipts bind Terms version/content hash/time. Drafts cannot be accepted. Existing users can review updated Terms without repeating setup. Terms and privacy choices stay separate from external-processing permission.
- Qwen download uses publisher SHA256 verification before atomic promotion, under the shared file lock; successful verification caches by metadata/hash. Invalid files are quarantined.
- Fixed Windows duplicate-ggml backend registry loading by resolving the loader directly from llama's ggml.dll. Actual generation validated it.
- Installer icon/description, per-user setup, English/Portuguese/Simplified/Traditional-Chinese options, Terms page and notices resources prepared. beforeBundleCommand and package-local refuse unresolved identity, drafts and third-party review.

## Evidence

Read `docs/reviews/2026-10-21-guided-multilingual-review.md`, `docs/benchmarks/2026-10-21-local-translation.md`, and `docs/reviews/2026-10-21-guided-verification.md`.

- Final native suite206 passed,0 failed,7 optional ignored,2 clipboard/audio mutation tests excluded. Frontend build73 modules and native release build passed.
- Actual Qwen translation: PT→EN, Chinese→EN, EN→Simplified/Traditional Chinese and PT, two passes. Warm second-pass206–286ms on RTX3080; initial cold calls up to10–11s, subsequent fresh process first call2.07s. Short fixtures, not a comprehensive quality ranking/native-speaker evaluation.
- Guided browser fixtures use synthetic IPC/legal docs: eleven behavior scenarios plus16 captures passed; ordinary/API fixture receipts updated separately. Refer to each results.json for final timestamp and source fingerprint.
- Independent reviews' checksum, consent-race, setup/remote bypass, normalized-form and acceptance-routing findings fixed; no material integration blocker remained in the reviewed code.
- Native candidate hash `39EE737BCB8174EFFEBD7F40D18A2EB805563C3CBC52FD47EB21C8EBCFAB67E9`, version still0.7.1. Not installed or an approved distributable.
- Third-party inventory685 Cargo packages,134 npm lock entries,1218 exact notices,130 unresolved inventory entries. Not an installer SBOM or compliance certificate. See `docs/legal/RELEASE-CHECKLIST.md`.

## Next actions and unresolved work

1. Keep public distribution blocked until publisher/contact arrangements, final Terms/Privacy and actual shipped third-party/model obligations are resolved. `legal/manifest.json` intentionally records draft/unapproved state. Do not mark it ready just to make packaging pass.
2. Review the staged UI/feature diff. Fresh-install/native permissions, upgraded acceptance, actual clipboard/remote insertion and live cloud account testing remain distinct from unit/browser evidence. Cloud tests currently use local fixtures; no provider key was supplied.
3. Qwen3-ASR remains an integration/benchmark candidate for Chinese dialects, not implemented. Only listed translation directions have actual-model smoke evidence; Cantonese/other targets need language-quality evaluation.
4. Preserve the previous unresolved Windows volume issue and real Windows-to-Mac acceptance work; no OS audio settings changed. Existing installed app remains the earlier version.
5. Finalize installer publisher/version/signing strategy and test real fresh install/repair/uninstall only after legal decisions. No Windows certificate, company or account was purchased/created.

## Reproduction and artifacts

- `scripts/verify-native.cmd`: native suite; `--build`: development executable; `--translation-bench`: compiles and stages matched backend DLLs then actual model test.
- `scripts/stage-native-backends.ps1`: selects the native build whose llama.dll hash matches release and stages its10 backend DLLs beside release/test executables. No global paths or installed app touched.
- `models/Qwen3-4B-Q4_K_M.gguf`:2,497,280,256-byte verified test cache, git-ignored; not copied to user app data. SHA256 `7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5`.
- `scripts/ui-smoke.mjs`: ordinary/API/guided fixture modes and evidence under docs/reviews.
- `scripts/check-legal.mjs --release`: EXPECTED FAIL until real release inputs resolved. `scripts/collect-third-party-notices.mjs`: repeatable locked/offline inventory, optional explicit model-license refresh.
- `legal/`, `docs/legal/`: draft documents, blank public identity manifest, copied notices and release review evidence. No private publisher identity or new persistent user credentials generated.
- Canonical handoff plus `docs/handoffs/2026-10-21-guided-multilingual.md` snapshot.

## Suggested skills

handoff-memory; systematic-debugging for failures; verify/computer-use for controlled native acceptance; Impeccable for UI; verification-before-completion; handoff. Review agents are user-authorized; retain single-file ownership. Use primary legal sources and qualified Portuguese advice for publisher/token/consumer duties; do not turn engineering checks into legal guarantees.

## Next-session prompt

```text
Lectus has an installed earlier API/platform build, plus uncommitted guided multilingual/translation/legal work now verified locally. Read docs/HANDOFF.md and docs/reviews/2026-10-21-guided-multilingual-review.md. The latest work is not installed; public packaging intentionally fails on unresolved publisher identity, draft legal documents and third-party review.
Files: legal/manifest.json, docs/legal/RELEASE-CHECKLIST.md, src-tauri/src/translation.rs, src-tauri/src/legal.rs, src/components/Onboarding.tsx, scripts/verify-native.cmd.
Use the current coding model at high reasoning for integration verification.
Skills: handoff-memory, systematic-debugging if needed, verification-before-completion, handoff.
Continue the user's privacy-preserving publisher decision and native acceptance work without publishing private name/address/tax information or bypassing the legal-release checks. Preserve all prior edits and verified evidence.
```
