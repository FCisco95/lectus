# Lectus browser fixture verification — 2026-10-21

## Result and scope

Ten interaction flows passed in headless Chromium against the current Vite frontend. Sixteen screenshots cover Home, General, Models and AI cleanup at 1100×800 and 720×540, each in light and dark themes. Page and content-container horizontal overflow checks passed for all screenshots.

This is a **browser fixture**, not a running Tauri app. Every IPC command is mocked. It does not prove real microphone capture, hotkeys, clipboard insertion, audio restoration, model loading or inference, provider credentials, membership, native window behavior, or Windows-to-Mac delivery. No production credentials, user configuration, clipboard, audio device or installed Lectus process was touched. The harness owns and stops a loopback Vite server and headless browser.

## Repeatable command

Run `node scripts/ui-smoke.mjs` with `playwright` available to Node. No app dependencies were added. An existing `playwright-core` installation can be selected through `PLAYWRIGHT_MODULE`; an existing Chromium binary can be selected through `PLAYWRIGHT_CHROMIUM_EXECUTABLE`. `UI_SMOKE_PORT` defaults to 4179 and Vite uses strict-port mode so it cannot silently attach to another server.

On this machine, the tested package was the existing npm cache package `C:/Users/joao_/AppData/Local/npm-cache/_npx/e78b33305587cb7c/node_modules/playwright-core`, and the browser was `C:/Users/joao_/AppData/Local/ms-playwright/chromium_headless_shell-1243/chrome-headless-shell-win64/chrome-headless-shell.exe`. These are environment inputs, not required machine-specific paths embedded in the harness.

Machine-readable results and PNG evidence are in [2026-10-21-ui-smoke](2026-10-21-ui-smoke/). Screenshot animations are disabled to capture the final rendered state.

## Behavioral evidence

1. Settings name entry retains focus and exact Unicode text across debounce/save. Voice activity detection responds to Space and Enter. Dark appearance updates the document theme and pressed state. Escape closes Settings and restores focus to its trigger.
2. Two overlapping autosave revisions serialize into a maximum of one in-flight mocked write. The latest URL persists. An edit made while focus-triggered save is in flight survives the stale saved revision, and a failed save survives focus refresh and later retries successfully.
3. History deletion first focuses Cancel, Cancel restores the clear trigger, and no deletion command occurs before confirmation. Confirm sends one clear command and displays the cleared state.
4. Model download progress persists when reopening the Models tab. A retained download error is shown with a retry action. Retrying returns to progress state.
5. Cloud connection check is disabled without a key. Mocked metadata success retains the explicit limitation that actual inference needs a dictation test. Missing selected model IDs and provider failures are visible.

Follow-up coverage after the final UX changes also passed: a delayed app-detection result cannot overwrite a manually edited profile; a cloud metadata result is discarded if cloud enablement changes while it is pending; and labeled Dictation/Dictionary controls support keyboard changes, word creation, replacement-rule edits and persistence.

Browser runtime errors and unexpected unmocked commands are also checked after each interaction flow.

## Visual inspection

The light and dark layouts keep navigation, titles, cards and primary controls aligned at both tested sizes. No horizontal clipping was observed. At 720×540, General rows stack and longer settings panels scroll vertically while the modal navigation and close control stay available. Home statistics and history remain readable in the compact viewport. Model readiness uses both text and color. The compact General panel shows the contextual-help summaries and destination card above the fold; lower settings require scrolling, which is expected for this layout.

The fixture initially reproduced a keyboard accessibility regression: closing Settings attempted to focus its trigger before the parent removed `inert`. The product owner changed focus restoration to the next animation frame; the regression test then passed. The history empty-state expectation was updated to the newly added explicit “History cleared.” message.

## Limits

The model inventory contains two representative fixture cards, not an assertion about downloaded files. The cloud key is a synthetic string and all provider responses are local fixtures. Screenshot text and model state are synthetic. Screenshots are inspected for layout and visible rendering, not a full WCAG contrast audit. Native execution and provider smoke tests need separate receipts.

## Final frozen-source receipt

Completed at 2026-10-21T17:24:04.930Z. All scenarios passed. Frontend SHA256 before and after this run: 06153d6a16bb2f820b5b9d15a18c4ed4ee6dc09be0e3962383e81ed2bfbd1f07. The fingerprint covers 40 files (src/**, index.html and package-lock.json), with exact bytes and sorted relative paths. Ordinary, API and guided modes were executed once each in chronological order after the final contextual-help changes; all three have the same unchanged frontend fingerprint. Screenshots were regenerated by those runs. This does not fingerprint or validate the native backend.
