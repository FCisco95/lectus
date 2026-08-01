# Lectus — HANDOFF

_Last updated: 2026-08-01 (night) • Branch: `master` • Version: 0.4.0 (0.5.0 pending Track 1 sign-off)_

> **⏸ WAITING ON CISCO: mockup sign-off.** The v0.5.0 session ran two tracks.
> Track 2 (models, measured) is DONE and committed. Track 1 (native UX overhaul)
> landed its non-visual half; the visual restyle is gated on your approval of
> `docs/mockups/` — open `settings.html`, `pill.html`, `onboarding.html` in a
> browser (mac/win × light/dark toggles at the top of each page).

## This session's commits (all on master, in order)

1. `3bd3d65` fix(macos): paste main-queue hop + hotkey capture focus (previous session's live-debug diff)
2. `82e0dc2` docs: UI-overhaul mockups for sign-off
3. `0664f9d` feat(ui): hotkey glyphs (⌃⇧⌥⌘ via `formatHotkey()` in types.ts) + dead template cleanup (App.css, vite.svg, index.html title)
4. `eae03dd` feat(tray): mac template icons (36px, icon_as_template, re-asserted after set_icon), correct initial icon + tooltip both OSes, win left-click opens Settings
5. `ec4b32d` feat(macos): ActivationPolicy::Accessory (no Dock icon), settings titleBarStyle Overlay + hiddenTitle + transparent (via tauri.macos.conf.json), window-vibrancy Sidebar, tauri-plugin-os (`IS_MAC` = `platform()`), `<html>` gets `platform-mac|win`
6. `993d3ec` feat(onboarding backend): `accessibility_status` / `microphone_status` / `request_microphone_access` commands, tauri-plugin-process (relaunch), `Config.onboarding_completed` (first run shows settings window)
7. `7be12c9` feat(bench): per-model chat templates (Gemma/Qwen-ChatML/Granite) + clean_output, PT clip, TTFT visibility, warmup+best-of-3 (51 tests green)
8. `c362738` feat(models): **default whisper → ggml-base.bin** + bench report

## Track 2 — RESULT (measured on this M4, Metal)

Full tables: `docs/benchmarks/2026-08-01-model-bench-m4.md`.

- **Whisper: default switched tiny → base.** Tiny mis-hears PT ("quem está
  feira" for "quinta-feira"); base is correct at 168–248 ms (RTF ≤ 0.033).
  small = 6–8× cost, no gain over base; large-v3-turbo (f16/q8/q5) ≈ 4 s/clip
  on the M4 — disqualified here, **re-bench on the RTX 3080/Vulkan** next
  Windows session. Quantized turbo is *slower* than f16 on Metal.
- **Cleanup LLM: Gemma 3 1B stays.** Qwen3-1.7B slower AND under-cleans;
  Qwen3-0.6B 2× faster but barely cleans; Granite 4.0 1B produces garbage on
  Metal (llama.cpp runtime bug, arch is supported). Future A/Bs are one env
  var away: `LECTUS_CLEANUP_MODEL=<file> cargo test --release bench_cleanup -- --ignored --nocapture`.
- Candidate GGUFs remain in `~/Library/Application Support/ai.organic.lectus/models/`
  (≈5.8 GB — delete the turbo/Qwen/Granite files if disk matters).
- ⚠️ Cisco's live config still says tiny — select **Base** in Settings → Models
  (or delete config.json) to get the new default.

## Track 1 — state

**Landed (non-visual):** hotkey glyphs, tray (template icons need live eyeball),
Accessory policy, overlay titlebar + vibrancy plumbing (visually inert until the
restyle makes the sidebar translucent), plugin-os platform classes, onboarding
backend commands + first-run flag.

**Gated on mockup sign-off (next session's work):**
1. `tokens.css` (from `docs/mockups/tokens.mock.css`) + settings restyle + IA
   9→6 tabs (General/Dictation/Vocabulary/AI/History/Models&About), SVG icons,
   version via `getVersion()`, theme picker (`theme` config field).
2. Pill restyle (36px orb idle, capsule + red dot recording, shimmer-bars
   transcribing; lib.rs sizes → 64×64 / 240×64).
3. Onboarding UI (5 steps, uses the landed backend commands + relaunch).
4. Then: v0.5.0 bump (tauri.conf.json + Cargo.toml + package.json + About),
   delete docs/mockups/, HANDOFF refresh.

**Needs live M4 eyeball (code already landed):** menu-bar template icon in
light/dark + state switching; Dock icon gone; settings window opens + gets
focus under Accessory policy (fallback ready: flip Regular↔Accessory on
show/hide); traffic-light overlay position; first-run settings-window show.

**Windows regression check (next Windows session):** tray left-click, colored
icons still used, `cargo build` green after tauri-plugin-os/process additions;
re-bench turbo variants on Vulkan.

## Gotchas added this session

- `tauri.macos.conf.json` now carries a **full copy of `app.windows`** (platform
  config replaces arrays wholesale) — keep both files' window defs in sync.
- `set_icon` clears the mac template flag; `do_set_state` re-asserts
  `set_icon_as_template(true)` after every icon swap.
- Settings window is `transparent: true` on macOS only — its CSS must always
  paint an opaque content pane (sidebar translucency is the vibrancy zone).
- `microphone_status` uses AVFoundation via objc `msg_send!` — links the
  framework in lib.rs (`#[link(name = "AVFoundation")]` on the extern static).

## Build (unchanged)

macOS: `npm install && npm run tauri dev` · release `npm run tauri build`.
Windows recipe unchanged — see previous HANDOFF snapshot
(`docs/handoffs/2026-07-16-phase3-measured-build.md`) or git history of this
file for the VULKAN_SDK/vcvars env block.

## Suggested skills (next session)

- `run` — launch the app for the live-eyeball checklist above
- `verify` — synthetic dictation drive if the pill/hotkey paths need re-proving
- `commit` — land restyle chunks as they're approved
- `vercel:react-best-practices` / `simplify` — after the settings IA merge
