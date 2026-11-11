# AGENTS.md — Lectus

Lectus: voice dictation app (Tauri 2, Rust, React + TS). The repo was renamed from `chirp`; the crate, lib and binary still read `chirp` / `chirp_lib` / `chirp.exe`. Strategy and plans live in the cisco-brain vault, not here.

## Non-negotiables
- API keys live in env/config only. Never paste them into chat or commit them.
- `tauri-plugin-global-shortcut`/muda cannot register a bare modifier (e.g. Right Ctrl): it panics. Default hotkey is `Ctrl+Shift+Space`.
- Ask before changing the capture/injection or shortcut pipeline.
- Ask before pushing a `v*` tag (it publishes a release via `release.yml`) or replacing the installed app in `%LocalAppData%\Lectus`.

## Commands
- Frontend: `npm run build` (tsc + vite). CI uses Node 22.
- Native tests (Windows): `test-release.cmd` (`cargo test --release --lib` with the Vulkan/Ninja/vcvars env, target dir `C:\lt`).
- Native build: `build-release.cmd`. It also runs `deploy-local.cmd`, which copies the build over the installed app. For a build only, run `cargo build --release` in `src-tauri` with the same env.
- No git hooks exist. CI runs only on `v*` tags, so nothing gates a push: run the build and tests yourself.

## Where docs go
- `docs/HANDOFF.md`: current state, next step, blockers, open questions. About 80 lines, rewritten whenever state changes. Read it first. Archive an overlong one to `docs/handoffs/archive/`, never delete it.
- `docs/handoffs/`: a dated snapshot only for a major event.
- Plans, specs, audits, mockups: temporary. Delete them once the outcome is in `docs/HANDOFF.md` or the code. Git history is the archive.
- Guides users read (`docs/REMOTE-DICTATION.md`, `docs/LOCAL-API.md`) and benchmark results the code cites (`docs/benchmarks/`) stay.
- Generated output (logs, screenshots, eval runs, diagnostics, downloaded models) is never committed. Delete it when the run is done.
