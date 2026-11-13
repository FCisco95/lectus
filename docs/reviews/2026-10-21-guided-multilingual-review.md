# Guided multilingual app and release preparation

Date: 2026-10-21. Local development work; not installed or distributed.

## Delivered

- Separate spoken/output language controls. Translation defaults off, with explicit local Qwen3 4B or configured cloud processing. Simplified Chinese, Traditional Chinese and Cantonese are separate targets. No automatic provider fallback.
- Failure-safe insertion: translation failures preserve original text in history when storage succeeds and insert nothing. Successful translated history retains original text/language and a Copy original action.
- In-app Help hub plus contextual information explaining purpose, setup and data flow for models, language, translation, cloud/CLI, microphone, insertion, remote dictation, API, membership and vocabulary.
- Guided first-run setup with progress, explicit permission action, legal review, shortcut persistence and a review screen. A rejected finish remains visible. The user explicitly approved deferring microphone/dictation until setup completes.
- Backend-owned setup completion, versioned/content-hashed local Terms receipts and current-Terms review for existing users. Draft terms cannot be accepted. API and remote insertion honor the setup/legal gate too.
- Separate configured-provider and Claude CLI processing acknowledgements. URL changes clear the provider choice; normalized backend state returns to the form. Checks are repeated at request admission; speech holds the authorized client coherently and disallows redirects.
- Optional Qwen model download verifies its published SHA256 under the same lock before promotion/readiness. Existing files verify with metadata-keyed caching. Invalid files are quarantined.
- Windows local-model backend registry collision fixed and verified with actual generation.
- Installer configuration now includes product descriptions/icon, per-user setup, English/Portuguese/Simplified/Traditional-Chinese choices, Terms page and legal/third-party resources. Actual bundling refuses unresolved legal drafts.

## Verification

| Check | Evidence |
|---|---|
| Frontend production build | Passed, 73 modules |
| Final native suite | 206 passed, 0 failed, 7 optional ignored; 2 clipboard/audio mutation tests excluded |
| Actual local translation | Five language-direction fixtures, two passes; all passed. Warm second-pass 206–286ms on RTX 3080; cold costs separately documented |
| Browser guided checks | Eleven guided scenarios and sixteen captures passed; all modes together:28 scenarios and40 captures, same frontend fingerprint |
| Native development executable | Release build passed; version still0.7.1, not a new public release |
| Release safeguard | `tauri bundle` ran beforeBundleCommand and refused with exit1 on draft publisher/terms/license state, before packaging |
| Independent review | Consent races, setup bypass, checksum promotion race, remote gate and normalized-form mismatch fixed; reviewer reported no remaining material integration blocker |

See [translation measurements](../benchmarks/2026-10-21-local-translation.md), [browser evidence](2026-10-21-guided-verification.md), [implementation plan](../plans/2026-10-21-guided-multilingual-setup.md) and [legal facts/checklist](../legal/RELEASE-CHECKLIST.md).

## Public identity and legal status

The user confirmed Portugal and raised a personal-safety/privacy concern about public identity. Their private name was not copied into public-facing legal files. `legal/manifest.json` intentionally has no public publisher name or email. No company/account/domain was created and no third party was contacted.

An independent developer route is possible, but applicable Portuguese online-provider/consumer/data-protection duties are not waived by using a product name or adding a checkbox. Publisher/contact arrangements, tax/activity obligations and the token-based membership model need scoped professional review before public distribution. No immunity-from-claims or compliance-complete promise is made.

Terms/Privacy are factual drafts. Third-party inventory covers685 Cargo packages and134 npm entries with1,218 exact copied notices.130 entries remain unresolved, mainly absent optional/build packages or source-only license headers; this all-target inventory is not a proven installer SBOM. Actual shipped obligations, including Gemma terms and any MPL-covered components, still require reconciliation. Required notices are prepared and exposed through Help after packaging; collection alone is not legal sign-off.

## Remaining limits

- No new installer was produced, published or applied. The running app remains the earlier user-installed API/platform build.
- Cloud translation has mock request/authorization/error coverage, not a live account-quality test; no cloud key was supplied.
- The Qwen translation model is distinct from Qwen3-ASR. Chinese speech currently uses existing Whisper/custom speech-provider support; a native Qwen3-ASR engine is not integrated.
- Cantonese and other target options have prompt/routing coverage; only the five listed translation directions received the real-model smoke test. Native-speaker quality evaluation remains needed.
- Fresh-install, upgrade/uninstall, signed Windows identity and macOS notarization remain release acceptance tasks after the publisher/legal decisions. Browser mocks do not establish real microphone, clipboard, cross-device insertion or accessibility permission acceptance.
- The earlier intermittent Windows volume problem and real Windows-to-Mac insertion remain open.

Final frontend verification: ordinary10/API7/guided11 scenarios and40 captures, all passed. Final receipt2026-10-21T17:25:12.526Z. All runs report unchanged frontend SHA25606153d6a16bb2f820b5b9d15a18c4ed4ee6dc09be0e3962383e81ed2bfbd1f07.
