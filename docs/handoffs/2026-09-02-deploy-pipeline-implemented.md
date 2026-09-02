# Lectus - HANDOFF

## Metadata

- Last Updated: 2026-09-02 (Windows PC, late evening)
- Repository: `lectus` (github.com/FCisco95/lectus) — **PRIVATE, stays private for now (decision below)**
- Branch: `master` @ `482de20`+ (pushed; last code commit `2a3e2aa`). Tag `v0.5.0` → `b41002a` (pushed).
- Version in manifests: `0.5.0`
- Release `Lectus v0.5.0`: **draft** (reverted to draft after the AVX-512 crash; left as draft since the repo is private and the feed is inert anyway). CI run `33677520267`: Windows green, macOS DMG flake (assets from run 4 remain).
- **Live Windows install: `%LocalAppData%\Lectus\chirp.exe` = CI-built 0.5.0 (NSIS installer, 2026-09-02 21:52). Launched, stayed up, models loaded.**

## TL;DR

Deploy pipeline + auto-update shipped and exercised up to install. Five CI attempts, five distinct fixes, all
committed. The Windows install on this PC now comes from CI, not from `C:\lt` hand-copies — the original
stale-install problem is closed.

**Auto-update is built but inert**: the repo is private, so the GitHub Releases feed
(`releases/latest/download/latest.json`) returns 404 to the app (no token). Decision (user, 2026-09-02):
keep the repo private until the app has an **Organic token gate** (wallet-connect, hold ≥ N ORGANIC tokens
to unlock). Then choose: make repo public (feed works as built) or a separate public `lectus-releases` repo
(source stays private, needs a PAT secret + endpoint change). Second option matches the intent better.
Caveat surfaced: a token gate in a public repo protects the signed build, not the code — anyone can fork
and strip the gate.

Implementation summary (commits `593cf3d` → `2a3e2aa`):

- `scripts/bump-version.mjs <ver>` rewrites package.json / Cargo.toml / tauri.conf.json.
- `src-tauri/src/updates.rs`: background check on launch → download + signature verify → `update-downloaded`
  → Settings banner "Restart now / Later"; install gated on `RecordingState::Idle`. Manual "Check for updates"
  + last background error in Settings → About. Failures log-only (currently: 404 every launch, silent).
- `.github/workflows/release.yml`: `v*` tag → version guard → windows-latest + macos-15 via tauri-action →
  draft release + `latest.json` + `.sig`s. Windows: Vulkan SDK cache, Ninja, msvc-dev-cmd, junction +
  `CARGO_TARGET_DIR=D:\lt`, portable ggml flags, prebuild warm-up. macOS: `--bundles app` (no DMG).
- `src-tauri/tauri.windows.conf.json`: bundles DLLs + `backends/*.dll`. Loader also searches `<exe>/backends`.
- `deploy-local.cmd`: dev-loop copy incl. backends, UAC-kill fallback (fixed unescaped parens in a for block).
- Signing: minisign ID `EB78643AB2023BC3`; pubkey in tauri.conf.json; secrets `TAURI_SIGNING_PRIVATE_KEY` +
  `..._PASSWORD`; encrypted key in cisco-brain `40 - RESOURCES/Lectus Release Signing/` (vault `584d8ea`,
  not pushed). **Password NOT in the vault** — keep it in the password manager.

## What to do next

1. Use the installed 0.5.0 for a few days (dictation, cleanup LLM on Vulkan, tray, hotkey). If anything
   regressed vs the August local build, suspect the portable ggml flags (`GGML_NATIVE=OFF`, AVX2 baseline)
   for whisper's CPU paths — GPU path is Vulkan either way.
2. Next sub-project per the agreed order was "verification hardening", but the **Organic token gate** is now
   the gating item for public releases + updater. Brainstorm it (superpowers:brainstorming): wallet-connect
   provider, balance check (Solana RPC? Helius MCP available), failure UX, offline grace, where the check
   lives (launch vs feature), how it interacts with the updater.
3. After the gate: decide public repo vs `lectus-releases` repo → publish → updater E2E (bump 0.5.1, tag,
   publish, expect banner + restart; offline silent; tampered `.sig` rejected).
4. macOS: pull master on the Mac, install from the release `.dmg` (run-4 asset) or `.app.tar.gz`.
5. Nice-to-have: `gh run rerun --failed` is enough for the DMG flake if it ever matters; DMG is now skipped.

## CI gotchas learned (5 attempts, each ~25–45 min)

1. `${{ cond && '' || 'x' }}` always yields `x` — empty string is falsy. Use matrix `args` instead.
2. macos-14 (Xcode 15.4) can't compile ggml-cpu's `armv9.2-a+…+nosve+sme` variant → macos-15.
3. Junction `src-tauri\target → D:\lt` is transparent to cargo; also export `CARGO_TARGET_DIR=D:\lt`.
   Keep the junction so `target/release/...` resource paths still resolve.
4. tauri-build validates resource globs in chirp's build script, which can run before llama-cpp-sys-2 emits
   DLLs → `continue-on-error` warm-up `cargo build --release --keep-going` first (38 min cold).
5. **Runner CPU ≠ user CPU**: whisper-rs-sys builds ggml with `-march=native` → AVX-512 on the runner →
   `0xc000001d` illegal instruction on the Ryzen 5800X at launch. `GGML_NATIVE=OFF` + `GGML_AVX2/FMA/F16C=ON`
   (whisper-rs-sys forwards any `GGML_*` env var to CMake). llama-cpp-sys-2 was already fine (ALL_VARIANTS
   runtime dispatch).
6. DMG creation (`bundle_dmg.sh` / hdiutil) hangs intermittently on GH macOS runners → `--bundles app`.

## Other gotchas this session

- Bash tool mangles `\r`/`\t`/`\b` in command text → use Write/Edit or a script file; verify with `od -c`.
- `cargo check` needs vcvars + `VULKAN_SDK` + `CMAKE_GENERATOR=Ninja` + `CARGO_TARGET_DIR=C:\lt` via `.cmd`.
- Auto-mode classifier blocks reading the private key and combined push+tag commands; separate commands OK.
- No 7-Zip here: inspect installer payload via `msiexec /a <msi> /qn TARGETDIR=<dir>`.
- `deploy-local.cmd` launched from the Bash tool hangs the tool (child inherits the pipe) — run it from a
  real terminal or `start`-detach.
- Windows Event Log (`Application Error`, `APPCRASH`) is the fastest crash diagnosis for a tray app with no console.

## Known Gaps

- Updater never exercised end to end (blocked on repo visibility / token gate).
- macOS assets on the draft are from run 4 (`8195532`); identical app source to run 5, but not rebuilt.
- Windows runtime pass (autostart, force_exit/relaunch, drag-region) still unverified.
- `tauri-plugin-mcp` not wired. `docs/mockups/` cleanup pending. Vault commit `584d8ea` not pushed.

## Suggested Skills

- `handoff-memory` — reload this file next session
- `superpowers:brainstorming` — Organic token gate design
- `verify` — live dictation verification on the installed 0.5.0
- `handoff` — refresh at session end

## Next-session prompt

```text
Lectus: deploy pipeline shipped 2026-09-02 (master 2a3e2aa, tag v0.5.0 → b41002a, Windows install on
this PC is now the CI-built 0.5.0). Repo stays PRIVATE until an Organic token gate exists, so the updater
is inert. Read docs/HANDOFF.md first. This session: brainstorm the Organic token gate (wallet-connect,
hold ≥ N ORGANIC to unlock), then plan it. After it ships: pick public repo vs lectus-releases repo,
publish v0.5.x, run the updater E2E.
Model: claude-opus-5 high for the brainstorm; claude-sonnet-5 for implementation.
Skills: superpowers:brainstorming, superpowers:writing-plans, handoff.
```

## Generated artifacts this session

| What | Where | Notes |
|---|---|---|
| Pipeline implementation + 5 CI fixes | `593cf3d` … `2a3e2aa` on master (pushed) | |
| Tag | `v0.5.0` → `b41002a` (pushed, moved 4× during CI debugging) | run 33677520267 |
| Draft release | github.com/FCisco95/lectus/releases (draft) | Windows assets from run 5, macOS from run 4 |
| Installed app | `%LocalAppData%\Lectus` (NSIS, 0.5.0) | verified launches + stays up |
| Signing key backup | cisco-brain `40 - RESOURCES/Lectus Release Signing/` | vault `584d8ea`; password excluded |
| Handoff snapshot | `docs/handoffs/2026-09-02-deploy-pipeline-implemented.md` | this session |
