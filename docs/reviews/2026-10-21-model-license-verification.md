# Gemma model-agreement UI verification — 2026-10-21

**9 behavioral scenarios and 8 screenshot/overflow checks passed.** Final receipt: [results.json](2026-10-21-model-license-smoke/results.json), timestamp `2026-10-21T18:05:07.645Z`.

Frontend fingerprint before and after the run was identical: `cd41ae60e967d937f83f6af0476d291f91bab9bfb63291fd272ce42e09a04852` across 41 files (`src/**`, `index.html`, `package-lock.json`). Native/backend files are outside that fingerprint.

## Reproduction and boundaries

Set `UI_SMOKE_MODEL_LICENSE=1` and run `node scripts/ui-smoke.mjs`. Leave the API/guided flags unset or zero. Reuse an existing Playwright installation via `PLAYWRIGHT_MODULE` and optional browser path via `PLAYWRIGHT_CHROMIUM_EXECUTABLE`; no permanent dependencies were added.

All Tauri commands use synthetic in-memory fixtures. No actual Google/model agreement was accepted or revoked, no user receipt/configuration changed, no real model downloaded, no microphone/clipboard opened and no cloud or other external browser request occurred. Non-loopback browser requests are intercepted and asserted absent. Only the harness-owned Vite server and headless browser are launched; both were stopped after the run.

The test proves frontend gating and feedback against the stated command/event contract. It does not prove native model-use authorization, actual receipt storage, final bundled-document completeness or legal enforceability.

## Passed checks

1. Choosing Local cleanup fetches model-agreement status without accepting or downloading. The checkbox starts unchecked, acceptance stays disabled until checked, and checking alone sends nothing. Explicit acceptance sends exactly the displayed fingerprint. It unlocks the Download action without starting a download or enabling cloud permission.
2. Both document disclosures open by keyboard. A thirty-paragraph synthetic Terms payload is rendered in full, including its last paragraph; the focusable document region scrolls by keyboard. The incorporated policy payload also renders exactly. These synthetic documents contain no real publisher or user name and are not presented as an actual agreement.
3. A previously downloaded model remains unavailable until separate acceptance. Document-load failure stays gated and exposes explicit Retry. Stale-fingerprint rejection does not unlock the model; the explicit Reload model terms button loads the new fingerprint with an unchecked choice, while Download remains disabled during loading. Acceptance and download failures preserve the appropriate separate state and allow retry.
4. A failed revocation preserves the existing accepted state and shows an error. Retrying successfully disables Download and clears the checkbox. Local remains the chosen engine; provider and Claude permissions remain off. Agreement-change events update the gate for an existing downloaded file and reset an outstanding unchecked/checked choice appropriately.
5. A newer revocation event wins over an older, delayed successful acceptance response. The UI remains unaccepted and Download remains disabled, with no stale “accepted” action available.

The nine individual scenario names are retained in the JSON receipt. Browser runtime exceptions, unexpected commands and attempted external browser requests are asserted absent for each case.

## Visual inspection

All eight PNGs were inspected: accepted/unaccepted states, light/dark, at 1100×800 and 720×540. No horizontal clipping/overflow was observed. Checkbox, disclosure links, acceptance/revocation and Download states remain distinguishable and usable. The panel scrolls vertically; screenshots align its agreement section near the top. Entrance animations are disabled during capture.

The final product pass removed the duplicated accepted confirmation. The fixture now asserts exactly one accepted confirmation after the action, and the refreshed accepted-state captures show the single persistent status. No product code was changed in this verification lane.

One initial assertion checked a checkbox immediately after dispatching an event, before React had rendered the update. It was changed to wait for the expected unchecked DOM state; the final full run passed without weakening the state assertion. The transient failure screenshot was removed after verifying its absolute path remained inside this test's owned evidence directory.
