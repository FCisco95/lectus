# Lectus — HANDOFF

_Last updated: 2026-08-01 (evening) • Branch: `master` • Version: 0.4.0_

> **🔴 LIVE-TEST RESULT (came in after the audit): dictation is BROKEN in real use on the Mac.**
> Cisco reports: (1) pressing the pill/"speak" button makes it stop working (echoes an old Windows
> failure), and (2) the Settings hotkey-capture button won't accept/save a key — so there is no
> working trigger at all. The headless verification below all passed; the interactive paths did not.
> **Next session: open this repo and paste the debug prompt in the section
> "NEXT SESSION — live debug prompt" at the bottom of this file.** Most likely first suspect:
> Accessibility not granted (or granted but app not relaunched) — but do NOT assume; follow the
> prompt's one-step-at-a-time loop with Cisco at the keyboard.

> **Repo state:** macOS cross-platform audit + fix pass done on the M4 Mac. All changes are in the
> working tree (not yet committed). Windows behavior untouched — every fix is either macOS-gated
> (`cfg`, `tauri.macos.conf.json`, `Info.plist`) or platform-neutral text/comments.
> Previous session state (Windows Phase 3, measured): see `docs/handoffs/2026-07-16-phase3-measured-build.md`.

## TL;DR — macOS parity session (2026-08-01)

The README's "cross-platform (macOS + Windows)" claim was aspirational — this session made it
largely true and **verified it by running the app on the M4** (dev binary, `tauri dev`, AND a
release `.app` bundle):

1. **Standalone launch was broken on macOS** — llama's mandatory dynamic-link dylibs are referenced
   via `@rpath` but the binary had **no LC_RPATH**; it aborted in dyld outside `cargo run`.
   Fixed in `build.rs` (rpath = `@executable_path` + `@executable_path/../Frameworks`).
2. **Release bundling was broken** — Tauri's default `MACOSX_DEPLOYMENT_TARGET=10.13` makes ggml's
   `std::filesystem` (needs 10.15) fail to compile. Fixed: `bundle.macOS.minimumSystemVersion: "11.0"`.
   ⚠️ If you ever hit the 10.15 error again after config changes, `rm -rf target/release/build/{whisper-rs-sys,llama-cpp-sys-2}-*`
   — the cmake cache pins the old target.
3. **macOS DLL-shipping equivalent solved + verified** — new `tauri.macos.conf.json` ships the four
   linked dylibs (`libggml-base.0`, `libggml.0`, `libllama-common.0`, `libllama.0`) into
   `Contents/Frameworks` and the ggml backend modules (`libggml-metal.so`, `libggml-cpu-*.so`) into
   `Contents/Resources/backends`; `ai/local_llm.rs` fallback now also searches `../Resources/backends`.
   **The bundled `Lectus.app` launches standalone and runs Metal.**
4. **Whisper Metal confirmed live at runtime** — `whisper_backend_init_gpu: using Metal backend`,
   Apple M4, embedded metallib. Warmup 90 ms (debug) / 427 ms (bundled release first-run).
5. **Cleanup LLM (Gemma 1B) verified on Metal** — `bench_cleanup` on the M4: EN cleans correctly,
   PT stays PT. **Warm latency 310–913 ms** (vs 111–166 ms on the RTX 3080) — still under the
   dictation-tolerable line but ~3–6× slower; revisit if it feels laggy in live use.
6. **Accessibility permission now prompts** — `AXIsProcessTrustedWithOptions(prompt)` shows the
   system dialog and lists Lectus in Settings → Accessibility (before: silent stderr + bail; the
   user had no discoverable path to grant it). App still degrades gracefully to pill-click if denied.
7. **Mic permission** — new `src-tauri/Info.plist` with `NSMicrophoneUsageDescription` (merged into
   the bundle, verified in the built .app's Info.plist). Without it TCC kills the bundled app on
   first mic access — and Lectus opens the mic at launch (pre-roll ring).
8. **Pill transparency** — `macOSPrivateApi: true` + tauri `macos-private-api` feature (transparent
   window is an opaque white square otherwise). Note: blocks a Mac App Store build; fine for
   direct/DMG distribution.
9. **Per-app profiles now work on macOS** — `context::foreground_app()` implemented via
   NSWorkspace `frontmostApplication` (was hardcoded `None`, Apps panel dead). Returns lower-cased
   localized name ("visual studio code") vs Windows exe name ("code.exe") — substring rules like
   "code" match both.
10. **Cross-platform hygiene** — benches no longer hardcode `%APPDATA%` (run on both OSes now);
    Settings UI hides "Typed keystrokes" on macOS (no SendInput twin yet — every mode resolves to
    clipboard paste there); hotkey hints say Option/Cmd on Mac; error strings de-Windows-ified.

**Verified green on macOS:** `cargo build`, 47/47 unit tests, `npm run tauri dev`, standalone debug
binary, `npm run tauri build` (.app + .dmg), bundled .app standalone launch, model auto-downloads
into `~/Library/Application Support/ai.organic.lectus/models/`, VAD model fetch, `bench_cleanup`.

## Live human debug session (2026-08-01, same day, after the audit)

The audit's headless checks all passed but live use was broken in two ways the audit could not see.
Both were root-caused with Cisco at the keyboard, fixed, and **re-verified live** (5 complete
dictation cycles in the log, text landing in real apps, history populated):

1. **Every dictation crashed the app at the paste step** (`EXC_BREAKPOINT` / SIGTRAP,
   `dispatch_assert_queue_fail` inside `TSMGetInputSourceProperty`). enigo resolves the
   layout-dependent 'v' keycode via TIS/TSM, which macOS asserts must run on the main thread —
   the pipeline runs on a tokio worker. Capture and transcription worked; the process died mid
   Cmd+V. Four identical crash reports in `~/Library/Logs/DiagnosticReports/chirp-*.ips`.
   **Fix:** `injection/clipboard.rs` — `simulate_paste()` hops to the main queue via `dispatch2`
   (new direct macOS dep, was already in the lockfile) with a `pthread_main_np` guard; the
   clipboard save/confirm/restore stays off-main. Windows path untouched.
2. **Hotkey capture in Settings was dead** — `HotkeyCapture.tsx` listens for `onKeyDown` on the
   button, but macOS WebKit does not focus a `<button>` on click, so key events never reached it
   (works on Windows/Chrome — hence the platform split). **Fix:** explicit
   `e.currentTarget.focus()` in the click handler. Capture + save + hook rearm verified live.

**Why the audit missed both:** neither is reachable headless — the crash needs a real dictation
reaching injection, and the focus quirk needs a real click + keypress in the webview.
Measured on the M4 (debug build): transcription 111–186 ms, injection ~290 ms per dictation.

## What still needs a HUMAN test on macOS (not verifiable headless)

_Updated after the 2026-08-01 live debug session:_

1. ~~Grant permissions + dictation cycle~~ **DONE live** — mic + Accessibility granted, pill-click
   dictation works end-to-end (pre-roll 500 ms → capture → transcribe → inject → history), and
   hotkey capture in Settings now saves a key (see live-debug section above). Remaining sub-check:
   a full **hold-to-talk** run with the physical key across sleep/wake (CGEventTap timeout gap below).
2. **Clipboard-paste injection breadth** — verified into the dictation test targets; still worth one
   explicit pass in iTerm/Terminal (bracketed paste) and Slack, plus confirming the prior clipboard
   restores ~200 ms later.
3. **Pill overlay polish** — transparency/drag confirmed in use; tray icon idle/recording/transcribing
   switching and template-icon polish still to eyeball.
4. **Apps panel on macOS**: add a profile matching e.g. "code", dictate into VS Code, check
   `app profile matched` log line.
5. **PT dictation quality + cleanup latency feel** on the M4 (A5 twin for the Mac).

## Known gaps / decisions left open (macOS)

- **No typed-keystroke injection on macOS** — needs a `CGEventKeyboardSetUnicodeString` twin of
  SendInput for terminal-safe, clipboard-free injection. UI hides the option meanwhile.
- **Dock icon shows** — a tray+pill app usually wants `ActivationPolicy::Accessory` (hides Dock
  icon). One-liner in setup, but product decision → Cisco.
- **CGEventTap timeout robustness** — macOS can disable an event tap it deems slow
  (`kCGEventTapDisabledByTimeout`); the callback doesn't re-enable it (macOS twin of the Windows
  B5 watchdog, which is a no-op on mac). Callback is tiny atomics so risk is low; fix belongs in
  the hook if hold-to-talk ever dies after sleep.
- **`PersistentCapture` `unsafe impl Send/Sync`** — comment justifies it for WASAPI; on macOS
  (CoreAudio) cpal's Stream is also `!Send`. Same "never touched after construction" argument
  applies, but it's unaudited on macOS. Flagged, not changed (pipeline architecture).
- **Bundle glob fragility** — `tauri.macos.conf.json` resources use
  `target/release/build/llama-cpp-sys-2-*/out/backends/*.so`; if multiple stale hash dirs
  accumulate the glob can pick up old modules. `rm -rf target/release/build/llama-cpp-sys-2-*`
  before a shipping build if in doubt.
- **Unsigned/unnotarized** — the .dmg is ad-hoc; Gatekeeper will quarantine on other Macs.
  Signing + notarization is a separate shipping task.

## How to build — macOS

```bash
cd ~/Desktop/projects/lectus
npm install
npm run tauri dev        # dev (Metal; no SDK setup needed — unlike Windows/Vulkan)
npm run tauri build      # release: .app + .dmg under src-tauri/target/release/bundle/
# bare cargo check/test needs dist/ once: npm run build
cargo test               # in src-tauri — 47 tests
cargo test --release bench_cleanup -- --ignored --nocapture   # needs Gemma GGUF in app-data models/
```

## How to build — Windows (unchanged)

```cmd
REM Kill chirp.exe first — running exe locks the binary.
set VULKAN_SDK=C:\VulkanSDK\1.4.350.0
set PATH=C:\VulkanSDK\1.4.350.0\Bin;%PATH%
set CMAKE_GENERATOR=Ninja
set CARGO_TARGET_DIR=C:\lt
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
cargo build --release   REM in src-tauri; exe at C:\lt\release\chirp.exe
```

## Shipping gotchas (BOTH platforms now documented)

- **Windows**: llama-cpp-2 dynamic build emits `{ggml,ggml-base,llama,llama-common}.dll` +
  `backends/` (ggml-vulkan.dll, ggml-cpu-*.dll) — bundled builds must ship them next to the exe
  (`load_backends_from_path` fallback).
- **macOS**: handled automatically now — `build.rs` rpaths + `tauri.macos.conf.json` frameworks/
  resources + the widened backend search in `ai/local_llm.rs`. Verified working in the built .app.

## What to do next (priority order)

1. Human permission-grant + live dictation test on the Mac (list above) — the only thing between
   "verified headless" and "works".
2. Commit this session's diff (16 files + 2 new: `Info.plist`, `tauri.macos.conf.json`).
3. On the Windows machine: pull, confirm `cargo build` + bundling still green (expected no-op, but
   the Cargo.toml/tauri.conf edits deserve one Windows build to be sure).
4. Decide: Accessory activation policy (hide Dock icon)? Template tray icons for macOS?
5. macOS typed-injection twin (CGEventKeyboardSetUnicodeString) if terminal dictation via paste
   annoys.
6. Then back to the product track: A5 pt A/B, B6 context-awareness, packaging/onboarding UX.

## Suggested skills (next session)

- `superpowers:systematic-debugging` — live dictation IS misbehaving on the Mac; this is the session
- `run` — launch and drive the app for the live loop
- `parallel-debugging` — if the two symptoms (pill pipeline dies / hotkey capture dead) turn out to
  have independent root causes
- `commit` / `commit-push-pr` — land fixes as they're confirmed
- `superpowers:writing-plans` — only after the Mac is actually usable: B6 context-awareness

## NEXT SESSION — live debug prompt (paste this to start)

```
Lectus is broken in live use on this Mac (M4) — I need a hands-on debug
session where YOU drive and I test. Two symptoms:

1. Pressing the pill / "speak" button: recording stops working right after
   I press it (similar to a failure we had on Windows). Either it never
   starts, or it starts and immediately dies.
2. Settings → Dictation: I cannot set up a hotkey. The capture button does
   not accept my key presses / never saves a key. So I have no working way
   to trigger dictation at all.

CONTEXT — read first:
- docs/HANDOFF.md — a macOS-parity audit was just done (2026-08-01).
  Headless checks passed (Metal live, bundle launches, 47 tests green) but
  NONE of the live interaction paths were human-tested.
- CLAUDE.md non-negotiables: ask before changing the capture/injection or
  shortcut pipeline architecture.

HOW TO RUN THIS SESSION — strict loop, one hypothesis at a time:
- Start the app yourself with full logs:
    RUST_LOG=debug npm run tauri dev 2>&1 | tee /tmp/lectus-live.log
- Then tell me ONE concrete action to perform at the keyboard, wait for me
  to say done, then read the log tail and state what actually happened
  before proposing anything. Never stack multiple test steps.

CHECK IN THIS ORDER (most likely first):
1. Permission state at startup: grep the log for the Accessibility prompt /
   "keyboard hook not installed" / "accessibility permission not granted".
   If the hook is not installed, walk me through System Settings → Privacy
   & Security → Accessibility (and Input Monitoring if needed) step by
   step, INCLUDING the restart of Lectus afterwards, then re-verify from
   the logs that the CGEventTap actually installed. Note: in dev the TCC
   entry may be attributed to the terminal/binary, not "Lectus" — tell me
   exactly which entry to look for.
2. Pill click path: after I click the pill, trace the log:
   hold-start → "pipeline: pre-roll contributed" → capture → transcription
   → injection. Find the exact stage that errors or resets state to idle,
   and show me the log lines that prove it. Suspects: injection failing
   (enigo Cmd+V needs Accessibility) and erroring the whole pipeline; or
   run_pipeline's atomic-claim rejecting because state never returned to
   Idle after a previous failed run.
3. Hotkey capture UI: with the Settings window focused, I'll press the
   capture button and then a key while you watch. Determine whether the
   keydown even reaches the webview (add temporary console/log output if
   needed — capture uses KeyboardEvent.code mapped through CODE_TO_KEY in
   src/components/settings/types.ts). Check whether the button loses focus
   (its onBlur resets capture), whether macOS delivers the modifier keydown
   to a webview at all, and whether save_config runs and the hook rearm log
   fires. If modifier-only keys never reach the webview on macOS, say so
   plainly and propose the smallest fix (e.g. capture via the existing
   CGEventTap instead of the DOM) — but ASK before touching the hook
   pipeline.
4. Only after 1–3 are diagnosed: fix, rebuild, and re-run the SAME live
   test with me to confirm, then update docs/HANDOFF.md with what was
   actually broken vs. what the audit missed.

Do not claim anything works until I have confirmed it live at the
keyboard. If a fix requires macOS permissions changes, always include the
restart step — a granted permission does nothing until relaunch.
```
