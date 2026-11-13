# Guided setup, help, translation and privacy UI verification

2026-10-21 — eleven behavioral scenarios passed, plus sixteen screenshot/overflow checks. Receipt: [results.json](2026-10-21-guided-smoke/results.json).

## Scope and reproduction

Run `UI_SMOKE_GUIDED=1 node scripts/ui-smoke.mjs` with Playwright available, or select an existing package/browser through `PLAYWRIGHT_MODULE` and `PLAYWRIGHT_CHROMIUM_EXECUTABLE`. On PowerShell, set `$env:UI_SMOKE_GUIDED='1'` before the Node command. This mode writes only the guided evidence directory; ordinary and API fixture modes remain separate.

All Tauri commands are mocked. Legal documents, publisher text, membership, microphone status, translation models and transcripts are synthetic fixtures. No real agreement was accepted, microphone opened, wallet connected, model downloaded, cloud request made, clipboard modified or user configuration changed. The harness starts and stops only its own loopback Vite server and headless browser. These results do not establish native gate enforcement, real translation quality or legal readiness.

## Passed behavior

1. Help opens inside Lectus. A keyboard-operated disclosure opens its explanation. Help buttons route to Dictation, Models, General and Local API, without invoking the old external help command.
2. Simplified Chinese, Traditional Chinese and Cantonese are distinct output choices. Changing them preserves the spoken-language setting. Explicit cloud translation retains local speech recognition and does not silently grant cloud-processing permission. Local-model download failure is visible and retry can complete.
3. Provider processing permission starts unchecked and is cleared by changing the provider URL. Claude CLI permission is a separate unchecked choice; enabling it does not enable provider permission.
4. Draft Terms block Continue and expose no Accept Terms action. Ready synthetic Terms require an unchecked checkbox plus explicit acceptance. That action grants neither cloud nor CLI permission. Merely visiting the microphone step invokes no microphone-request command; the request is sent only after clicking its button. A rejected `complete_onboarding` keeps the review step visible and shows an error; a later successful retry opens Home.
5. Translated history exposes its original text and a dedicated Copy original action. Failed translation presents the original with an explicit no-insertion explanation. The copy assertion intercepts the command and never writes the system clipboard.

The final contract checks also passed: the fixture backend now returns normalized Config from save_config. A provider URL edit plus acknowledgement within one debounce submits checked=true, receives normalized false, and the visible checkbox returns unchecked while Saved is shown; a later separate acknowledgement persists. Completed users with draft documents retain Home and can read the drafts. Completed users with published, unaccepted synthetic Terms get only legal review, then return to Home after acceptance without repeating onboarding or requesting microphone access. The app's initial get_legal_status invocation is explicitly asserted.

LegalDocuments was inspected for its privacy/acceptance copy. Terms acceptance remains separate from provider/CLI processing choices; no real publisher name or actual user document appears in these fixtures. The current implementation reads document content returned by the backend rather than inventing publisher identity in the component.

Unknown commands and browser runtime exceptions are asserted absent after each scenario. These are frontend tests: native authorization, microphone gating and actual provider-request suppression need the separate Rust/runtime receipts.

## Visual inspection

Sixteen PNGs cover Help, translation settings, Terms setup and setup review, in light/dark at 1100×800 and 720×540. All were inspected. No horizontal clipping or overflow was observed. Chinese option labels render; form labels and descriptions remain readable; small windows scroll vertically with navigation and primary controls available. Setup progress and disabled acceptance/Continue states are visible. Synthetic legal version strings identify these as fixtures, not screenshots of a published agreement. Entrance animations were disabled for final captures.

## Independent review finding

Code review identified that generic config updates could promote `onboarding_completed` from false to true, bypassing the dedicated completion command. This was reported to the backend owner for a focused fix and native regression test. The browser mocks do not establish that native fix; track its verification in the parent's native receipt and the legal implementation-facts update.
## Final contextual-help and notices pass

The final frozen-source run includes keyboard expansion/collapse and default-collapsed checks for microphone/insertion, remote dictation, speech models, AI cleanup, app profiles, local API, membership and vocabulary explanations. The third-party licenses button invokes the fixed mocked open_legal_notices command; both success and visible failure are verified. No actual license directory or browser/native app is opened by the fixture.


## Final frozen-source receipt

Completed at 2026-10-21T17:25:12.526Z. All scenarios passed. Frontend SHA256 before and after this run: 06153d6a16bb2f820b5b9d15a18c4ed4ee6dc09be0e3962383e81ed2bfbd1f07. The fingerprint covers 40 files (src/**, index.html and package-lock.json), with exact bytes and sorted relative paths. Ordinary, API and guided modes were executed once each in chronological order after the final contextual-help changes; all three have the same unchanged frontend fingerprint. Screenshots were regenerated by those runs. This does not fingerprint or validate the native backend.
