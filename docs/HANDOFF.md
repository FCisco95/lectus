# Lectus - handoff

Updated: 2026-11-19 (website built). Previous: 2026-11-18, third session (planning only). Branch: feat/remote-dictation. No tag, release or app replacement.
Previous handoff (access/Android Orca run): `docs/handoffs/archive/HANDOFF-2026-11-13-access-android-run.md`.

## State

- One-click installer: `src-tauri/windows/installer.nsi`, forked from the stock Tauri 2.11.2
  template (2d01631 stock copy, c1ac028 changes). One branded window, installs in place, opens the
  app; the updater's `/P /UPDATE /R` uses the same window; `/S` silent, `/NOLAUNCH` no launch.
- Signing hook ready (ba470dc, `tauri.signing.conf.json`); owner declined a paid cert, so unsigned.
- In-app discovery (a80a2f5): one spotlight per session and a sidebar "Get started" card.
- New this session:
  - **Clean-PC fix.** chirp.exe, llama/ggml and the backends import MSVCP140, VCRUNTIME140(_1)
    and VCOMP140 (OpenMP) at load time. A clean Windows lacks them and the installer did not
    ship them, so the app would not start there. `src-tauri/build.rs` now copies them from the
    VS `VC\Redist\MSVC` folder into `target/<profile>/`; `tauri.windows.conf.json` ships them.
    The installer deletes those copies when System32 has all four at the same or a newer
    version (this PC: system 14.51, bundled 14.44), since an app-local runtime would be forced
    on every DLL in the process.
  - **Vulkan.** chirp.exe also imports `vulkan-1.dll` at load time (whisper GPU backend). It
    comes with the graphics driver (Khronos discourages bundling it). In interactive modes the
    installer now stops with a translated message when `System32\vulkan-1.dll` is missing.
  - **Bundle guard** `scripts/check-windows-bundle.mjs` (beforeBundleCommand, tests in
    `.test.mjs`): reads the PE imports of every shipped binary and fails on a missing or
    redistributable DLL. It also picks the llama backends build that matches the root
    `ggml-base.dll`: `cargo build` and `cargo test` keep separate `llama-cpp-sys-2-*` dirs, and
    llama-cpp-sys-2 links root DLLs only when missing, so stale root DLLs fail the bundle.
  - **Publisher.** `bundle.publisher` "Bakehouse Labs", copyright "© 2026 Bakehouse Labs". The
    template falls back to the old `Software\organic\<product>` key and deletes it after
    writing the new one.
  - **Legal identity** (owner decision below): `legal/manifest.json` publisher is the individual
    João Francisco Vieira, trading name Bakehouse Labs, PT. Terms/Privacy identity sections,
    a Microsoft runtime clause and `legal/third-party/WINDOWS-RUNTIME.md` added.
  - **User guide** `docs/INSTALL-WINDOWS.md`: 3 steps, SmartScreen "More info > Run anyway",
    Smart App Control, Vulkan message, updates, uninstall, `/S`.

## Verification (this session)

- Side-by-side "Lectus Installer Test" builds, never published:
  - no-Vulkan variant: message shown, nothing installed;
  - legacy ("organic") 0.7.1 then 0.7.2 ("Bakehouse Labs"): every check-kept item passed, old key
    removed, Publisher updated, runtime DLLs installed;
  - legacy install in a custom folder: upgraded in place through the old key;
  - uninstall removes the runtime DLLs;
  - runtime choice: 0 app-local copies on this PC, 4 with a variant that sees no system runtime.
  - The real install snapshot stayed unchanged, `Software\organic\Lectus` included.
- Real-name title widths (GDI, real fonts, 96 DPI): widest is 323/392 px (PT failure title).
- `node --test scripts/*.test.mjs` 24/24; `test-release.cmd` 312 passed, 0 failed, 13 ignored.
- Not verified: the app starting on a clean PC (needs a clean VM; Windows Sandbox is not
  enabled here), `/R` and auto-launch, 150%/200% DPI on screen, discovery in native WebView2.
- Testing gotchas: from Git Bash set `MSYS_NO_PATHCONV=1`; a message box with only OK closes
  with WM_COMMAND id 2, not 1; the Bash tool mangles backslashes, so patch files from scripts.

## Owner decisions

- 2026-11-18: no paid code-signing certificate before sales. Revisit with revenue (Certum
  individual cloud, about EUR 139-209/year; Azure Artifact Signing excludes EU individuals).
- 2026-11-18: publisher = the owner as an individual in Portugal, trading as Bakehouse Labs. No
  company for Lectus yet; form one when revenue, partners or liability justify it. Organic
  (the token) should get its own entity, separate from Lectus, if it involves other people's
  money or partners. Paid plans should go through a Merchant of Record, but check
  `.context/lectus-access-android-20261113/billing-policy.md` first (Paddle bans vouchers,
  Lemon Squeezy bans crypto-related products).
- 2026-11-18: for the lag diagnosis only, the owner authorized installing a release build of
  Input Leap and Deskflow on the PC and the Mac. He also wants the two computers linked with
  one click, never by typing or guessing an IP.

## Release gates (open)

- `legal/manifest.json`: `identity_disclosures_reviewed` (address and NIF that DL 7/2004 art. 10
  asks online providers to show; a domiciliation address avoids publishing a home address;
  confirm with an accountant), `third_party_notices_reviewed` (includes the VC runtime
  redistribution right for the VS edition that builds the release), final Terms/Privacy
  versions, `publication_approved`.
- Smart App Control blocks unsigned installers with no override.
- No capture, injection or shortcut change, and no installed-app replacement, without the owner.
- Official ChatGPT/Claude login and real inference need native acceptance. macOS is uncompiled.

## Next

1. **Parked 2026-11-18 while the owner plans something new; do this first when Lectus resumes.**
   Clean-PC acceptance in Windows Sandbox. Owner enabled the feature 2026-11-18; it works after
   his next restart. Harness: `scripts/diagnostics/sandbox-acceptance/` (README has the two
   commands). A full test installer is already built at
   `%TEMP%\lectus-test-installers\LectusInstallerTest_0.7.1.exe`; rebuild it if the release
   build changed. Read `result.json`, the screenshots and `app-stderr.log`.
2. Owner: address/NIF choice and accountant check, then legal approval, then a real-name candidate.
3. Owner-run acceptance over the real install: auto-launch, wizard, `/R`, DPI.

## Input Leap follow mode (planned 2026-11-18, no code)

Plan: `docs/plans/2026-11-18-input-leap-follow.md`. Dictation should follow the computer Input
Leap controls. Recommendation: PC-only "Follow" destination that reads the Input Leap server
log (`switch from "A" to "B"`); the Mac stays on 0.7.1 Receive; destination locked at key-down.
Input Leap upstream was archived 2026-07-26 and has no supported active-screen interface on
Windows/macOS. Open: owner runs Step 0 of the plan (15 minutes, no code) to learn whether PC
Lectus sees the dictation key while Input Leap relays to the Mac. Building the prototype needs
the owner's approval (destination routing). Owner tried Send/Receive and found it "not
straightforward". His wider goal: one app sharing keyboard, mouse and microphone, more efficient
than Input Leap. Microphone decided: dictation only on both computers, so text is sent, never
audio. Engine choice (fork, own Rust engine, Deskflow) waits on a lag diagnosis. **Next session:
run "Next session: lag diagnosis and the address problem" in the plan** (installs authorized,
see Owner decisions). First network sample: median 4 ms, max 108 ms, no loss, in the plan.

## Website (built 2026-11-19, not deployed)

`website/`: static marketing site, no build step, no third-party requests. Home with live demos
(hero dictation in four apps, typing-versus-speaking race, hold-the-key tryout, clean-up,
vocabulary, languages, privacy flow, remote dictation, local API, install and updates), plus
`download/`, `changelog/`, `privacy/`, `terms/`, `404.html`. `website/README.md` has preview,
the release sync script and the placeholder list. Checked at 1440 and 390 px in headless Chrome,
with and without reduced motion; no console errors.
- The copy describes the next release (wizard, trial, installer, translation, local API). Publish
  it with that release, not before. Legal gates above apply to the site too.
- Open for the owner: price and payment route, final legal texts, Mac status, domain and host.

## Still open from the access/Android run (run_db09bc5a3181)

Auth and policy returned `outcome_unknown`; follow their Orca receipts and do not resend. The
Android channel question and full design approval are pending. See
`docs/handoffs/2026-11-13-access-android-coordination.md`, `docs/designs/2026-11-13-access-and-android.md`.

## Next-session prompt

```text
Lectus, feat/remote-dictation, read docs/HANDOFF.md first. The owner's current priority is the lag diagnosis in docs/plans/2026-11-18-input-leap-follow.md, section "Next session: lag diagnosis and the address problem": first make the Mac reach the PC by name instead of IP, then compare the Debug Input Leap build with a release build and with Deskflow, with numbers. Installs of those two are authorized for this purpose; review each download first and keep the current setup restorable. No Lectus capture, injection or shortcut change without asking.

Parked installer work, for when that is done: Installer now ships the MSVC runtime, checks for Vulkan, publishes as Bakehouse Labs; legal identity is the owner trading as Bakehouse Labs. Next: run scripts/diagnostics/sandbox-acceptance (Windows Sandbox is enabled; needs the owner's restart first), then legal disclosures and approval. Keep publication and installed-app gates closed.
```
