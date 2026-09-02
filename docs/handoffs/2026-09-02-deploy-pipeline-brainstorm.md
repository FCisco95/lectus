# Lectus - HANDOFF

## Metadata

- Last Updated: 2026-09-02 (Windows PC)
- Repository: `lectus` (github.com/FCisco95/lectus)
- Branch: `master`
- Version in manifests: `0.4.0` (bump to `0.5.0` is first task of next session)
- Live Windows install: `%LocalAppData%\Lectus\chirp.exe` — updated to the 2026-08-26 build this session, relaunched, user-tested OK

## TL;DR

Fixed stale-install bug: desktop icon ran the Jul 17 build from `%LocalAppData%\Lectus` while the Aug 26 rebuild sat unused in `C:\lt\release`. Copied fresh `chirp.exe` + `chirp_lib.dll` + ggml/llama DLLs into the install dir and relaunched (PID 59396 at the time). Root cause class: no deploy pipeline — build output and install dir diverge after every rebuild.

Then brainstormed (superpowers:brainstorming) the fix-permanently sub-project: **deploy pipeline + auto-update**. User approved the full design. Spec committed at `docs/superpowers/specs/2026-09-02-deploy-pipeline-design.md` (commit `f54c732`).

What to do next:
1. Review the spec (user already approved in brainstorm — confirm no changes).
2. Invoke `superpowers:writing-plans` → implementation plan for the spec.
3. Execute: version bump script → updater plugin → CI release workflow → `deploy-local.cmd`.

## Current State

- Install dir `%LocalAppData%\Lectus` now holds the 2026-08-26 build (`chirp.exe` 72012288 bytes, `chirp_lib.dll` added, ggml/llama DLLs unchanged from Jul 16 — identical to build).
- `docs/HANDOFF.md` + snapshot refreshed; design spec committed. Working tree otherwise has pre-existing `docs/HANDOFF.md` mod + untracked `build-release.cmd` from earlier session.
- App tested by user after the fix: OK.

## Key Decisions (2026-09-02)

- Improvement roadmap decomposed into 4 sub-projects, order agreed: **deploy pipeline → verification hardening → latency round 2 (A2) → feature depth**. Only #1 has a spec.
- Deploy approach: **tag-driven GitHub Actions releases + tauri background updater** (Approach A). Rejected manual-check-only (staleness returns) and continuous prereleases (CI waste).
- CI targets both platforms; Windows CI must replicate the Vulkan build recipe (see `docs/handoffs/2026-07-16-phase3-measured-build.md`, `build-release.cmd`).
- Update feed: GitHub Releases `latest.json` via `tauri-action@v0`; signing keypair to be generated (pubkey → `tauri.conf.json`, privkey → repo secrets + cisco-brain vault backup).

## Gotchas Learned This Session

- Running `chirp.exe` may be **elevated** — non-elevated `Stop-Process`/`taskkill /PID` gets `Access is denied`. Kill via tray quit or UAC-elevated taskkill. `deploy-local.cmd` must handle this.
- Desktop shortcut `Lectus.lnk` → `%LocalAppData%\Lectus\chirp.exe`; after any `build-release.cmd` rebuild, copy `chirp.exe` + DLLs from `C:\lt\release` into install dir or icon goes stale again (until pipeline ships).
- PowerShell exec policy blocks `npm.ps1` — use `cmd /c npm ...`.
- Sandbox blocked frontend build file access previously — run builds outside sandbox.

## Known Gaps

- Version still `0.4.0` in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.
- No `latest.json`/updater/signing/CI yet — all in the approved spec, unimplemented.
- Windows runtime pass (autostart, force_exit/relaunch, drag-region) still not verified post-v0.5.0 — sub-project #2.
- `tauri-plugin-mcp` not wired — sub-project #2.
- `docs/mockups/` cleanup pending.

## Suggested Skills

- `handoff-memory` — reload this file next session
- `superpowers:writing-plans` — turn the approved spec into an implementation plan
- `superpowers:executing-plans` — run the plan
- `superpowers:verification-before-completion` — before claiming updater/CI done
- `handoff` — refresh at session end

## Next-session prompt

```text
Lectus deploy-pipeline implementation session. On 2026-09-02 we fixed a stale-install bug (desktop icon ran Jul 17 build from %LocalAppData%\Lectus while Aug 26 build sat in C:\lt\release — fixed by copying fresh exe+DLLs into install dir; app relaunched, tested OK).

A design spec for "Deploy pipeline + auto-update" was brainstormed, APPROVED by me, and committed (f54c732) at docs/superpowers/specs/2026-09-02-deploy-pipeline-design.md. Read it first, plus docs/HANDOFF.md and build-release.cmd.

Design summary: bump version 0.4.0→0.5.0 across package.json / src-tauri/Cargo.toml / src-tauri/tauri.conf.json via new scripts/bump-version.mjs; add tauri-plugin-updater + tauri-plugin-process with background check on launch (silent fail offline, never restart mid-recording, banner in Settings + tray badge); tag-driven GitHub Actions release workflow (tauri-action@v0, matrix windows-latest + macos-14, Windows must replicate the Vulkan build recipe — Ninja, Vulkan SDK cache, msvc-dev-cmd, see docs/handoffs/2026-07-16-phase3-measured-build.md); tauri signer keypair (pubkey into tauri.conf.json, privkey into repo secrets + cisco-brain vault backup); deploy-local.cmd for dev-loop deploys (PID kill with UAC fallback, copy build→install dir, relaunch); Settings→About shows getVersion().

Also know: repo is FCisco95/lectus; running elevated chirp.exe resists non-elevated taskkill (UAC fallback needed); PowerShell exec policy blocks npm.ps1 — use cmd /c npm ...; sandbox blocked frontend build file access last time — run builds outside sandbox.

First step: I review the spec (already approved in brainstorm — confirm no changes), then invoke superpowers:writing-plans to create the implementation plan, then execute it.

Model: claude-sonnet-5 high (xhigh for the Windows CI workflow).
Skills: superpowers:writing-plans, superpowers:executing-plans, superpowers:verification-before-completion, handoff
```

## Generated artifacts this session

| What | Where it lives | Notes |
|---|---|---|
| Updated Windows install | `%LocalAppData%\Lectus\` | 2026-08-26 build copied over Jul 17 build |
| Design spec (committed `f54c732`) | `docs/superpowers/specs/2026-09-02-deploy-pipeline-design.md` | Approved; input to writing-plans |
| Handoff snapshot | `docs/handoffs/2026-09-02-deploy-pipeline-brainstorm.md` | This session |
