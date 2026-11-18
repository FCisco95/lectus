# Lectus - handoff

Updated: 2026-11-18, second session. Branch: feat/remote-dictation. No tag, release or app replacement.
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
  - uninstall removes the runtime DLLs.
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

1. Clean-PC acceptance in Windows Sandbox (owner enables it: admin and a reboot). Install a test
   build there, then copy a Vulkan loader into the sandbox to exercise the CPU fallback.
2. Owner: address/NIF choice and accountant check, then legal approval, then a real-name candidate.
3. Owner-run acceptance over the real install: auto-launch, wizard, `/R`, DPI.

## Still open from the access/Android run (run_db09bc5a3181)

Auth and policy returned `outcome_unknown`; follow their Orca receipts and do not resend. The
Android channel question and full design approval are pending. See
`docs/handoffs/2026-11-13-access-android-coordination.md`, `docs/designs/2026-11-13-access-and-android.md`.

## Next-session prompt

```text
Continue Lectus on feat/remote-dictation from docs/HANDOFF.md. Installer now ships the MSVC runtime, checks for Vulkan, publishes as Bakehouse Labs; legal identity is the owner trading as Bakehouse Labs. Next: clean-PC acceptance in Windows Sandbox, then legal disclosures and approval. Keep publication and installed-app gates closed.
```
