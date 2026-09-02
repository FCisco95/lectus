# Lectus — Deploy Pipeline & Auto-Update Design

- Date: 2026-09-02
- Status: Approved (brainstorm session, user approved all 3 sections)
- Repo: `FCisco95/lectus` (https://github.com/FCisco95/lectus)

## Problem

Build output (`C:\lt\release`) diverges from the installed app (`%LocalAppData%\Lectus`) — desktop icon ran a stale Jul 17 build while an Aug 26 build sat unused. Version manifests still say `0.4.0` despite v0.5.0 features shipping. No installer, no updater, manual DLL copies.

## Approach (chosen: A — tag-driven release + background updater)

Push `v*` tag → GitHub Actions matrix (Windows + macOS) builds, signs, publishes release + `latest.json` → app checks on launch, downloads in background, prompts restart-to-apply.

Rejected: B (manual check only — staleness returns in softer form), C (continuous prereleases — CI waste for solo use).

## 1. Version + in-app updater

- **Single-source version**: bump `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` to `0.5.0`. Add `scripts/bump-version.mjs <version>` rewriting all three.
- **Plugins**: `tauri-plugin-updater` + `tauri-plugin-process` (restart). Pubkey embedded in `tauri.conf.json` `plugins.updater`; endpoint: `https://github.com/FCisco95/lectus/releases/latest/download/latest.json`.
- **UX**: on app start (Rust setup, after windows ready) spawn background check → if update found, download + install silently → emit event to Settings window → banner "Update to vX ready — Restart now / Later". No modal blocking dictation. Settings closed → badge on tray menu item. Manual "Check for updates" button in Settings → About.
- **Failures**: check failure (offline/GitHub down) → silent, log only. Signature mismatch → discard, log. Never auto-restart mid-recording (gate on idle app state).

## 2. CI release workflow

- **Trigger**: push tag `v*` → `.github/workflows/release.yml`, using `tauri-apps/tauri-action@v0` (build, draft release, upload bundles + sigs, generate `latest.json`).
- **Matrix**:
  - `macos-14` (arm64): standard — `npm ci`, rust toolchain, tauri-action.
  - `windows-latest`: replicate Vulkan recipe — Ninja + Vulkan SDK (cache `C:\VulkanSDK` keyed on SDK version), env vars from `build-release.cmd`, `ilammy/msvc-dev-cmd` for vcvars, short-path target (`D:\`). Cross-check `docs/handoffs/2026-07-16-phase3-measured-build.md` during planning.
- **Signing**: one-time `tauri signer generate`; pubkey → `tauri.conf.json`; `TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` → repo secrets; private key backed up in cisco-brain vault.
- **Human gate**: draft release → review notes → publish → feed live.
- **Version guard**: workflow fails early if tag version ≠ `tauri.conf.json` version.
- **Risk**: first Windows CI run 30–45 min (Vulkan SDK + cold cargo cache); mitigated by `Swatinem/rust-cache` + SDK cache. Fallback if whisper-rs CI build flakes: Windows local build + manual artifact upload; macOS stays on CI.

## 3. Local deploy + testing

- **`deploy-local.cmd`** (dev loops): verify build exists → kill chirp by PID (normal kill, fallback UAC-elevated taskkill) → copy exe + DLLs `C:\lt\release` → `%LocalAppData%\Lectus` → relaunch.
- **Testing**:
  - Updater E2E: publish `v0.5.1-test` prerelease, confirm running 0.5.0 detects/downloads/restarts into it. Verify whether tauri-action excludes prereleases from `latest.json`; add explicit test feed if needed. Delete test release after.
  - Tampered artifact → updater rejects.
  - Offline launch → silent no-op.
  - First tag run observed end-to-end; artifacts spot-checked on both OSes.
- **Rollback**: reinstall previous NSIS installer from GitHub Releases. No auto-rollback (YAGNI solo).
- **Version in UI**: Settings → About reads `getVersion()` from Tauri metadata — no hardcoded string.

## Out of scope

Latency round 2, verification hardening, feature depth — separate sub-projects, brainstormed later in this order: deploy pipeline → verification hardening → latency → features.
