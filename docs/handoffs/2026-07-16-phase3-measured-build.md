# Lectus — HANDOFF

_Last updated: 2026-07-16 00:40 • Branch: `feat/phase3-whisperflow-ux` • Version: 0.4.0_

## TL;DR

Measurement-first session that executed the surviving plan from `docs/LECTUS-NEXT-PROMPT.md`:

1. **A1 measured — ASR was never the bottleneck.** RTX 3080 Vulkan, batch-1: tiny 39–55 ms, base 49–77 ms,
   small 124–147 ms on 5.4 s/11 s clips (bench `bench_latency`). **A2 (CUDA A/B) cancelled**; A4 (Parakeet)
   deprioritized to "fallback engine someday".
2. **B1 pre-roll** — always-on capture stream, 500 ms ring; first word never clipped. Fallback to
   per-dictation stream if mic fails at startup; stream migrates on mic change.
3. **A3 SendInput** — default injection is now typed unicode keystrokes (terminals work, clipboard never
   raced); auto/sendinput/clipboard picker in Settings → General.
4. **A7 anti-hallucination** — no_context, entropy 2.6 / logprob −1.25, suppress blank+NST.
5. **B5 hook watchdog** — WH_KEYBOARD_LL re-installed every 60 s (idle only); survives sleep/RDP.
6. **A6 Silero VAD** — whisper.cpp built-in VAD (threshold 0.6, 250 ms speech / 200 ms silence); measured
   zero latency cost, identical transcripts; ~0.9 MB model auto-downloads; toggle in General.
7. **B2 LOCAL CLEANUP LLM — cloud dependency dead.** Gemma 3 **1B** Q4_K_M in-process via `llama-cpp-2`
   (**dynamic-link + dynamic-backends mandatory** — static ggml collides with whisper-rs, LNK2005).
   Measured 111–166 ms warm (≤200 ms budget VALIDATED). 270M was too weak. **English prompts make small
   models TRANSLATE Portuguese input — fixed with a fully localized pt system prompt keyed off whisper's
   detected language** (now returned by `LocalWhisper::transcribe`).
8. **B4 per-app profiles** — foreground exe detection → per-app overrides for cleanup/tone/language/
   injection; new Settings → Apps panel with "Detect" helper.

All builds green; **47 unit tests passing**.
Commits: `3884ab6` (A1) → `4ccdbd8` (B1/A3/A7/B5) → `1ad3e13` (A6) → `491f31a` (B2) → `382a1a3` (B4).

## Product framing (user decision 2026-07-15)

Building toward **commercial product**: local-first free tier (whisper + Gemma 1B, fully offline) + paid
cloud tier (Groq/Claude cleanup, cloud Whisper) — Wispr-Flow-style, but offline is the differentiator.

## How to build

```cmd
REM Kill chirp.exe first — running exe locks the binary.
set VULKAN_SDK=C:\VulkanSDK\1.4.350.0
set PATH=C:\VulkanSDK\1.4.350.0\Bin;%PATH%
set CMAKE_GENERATOR=Ninja
set CARGO_TARGET_DIR=C:\lt
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
cargo build --release   REM in src-tauri; exe at C:\lt\release\chirp.exe
```

Benches: `cargo test --release bench_latency -- --ignored --nocapture` (ASR) and
`bench_cleanup` (LLM; `LECTUS_CLEANUP_MODEL` env overrides the GGUF).

## Shipping gotcha (IMPORTANT for the installer)

llama-cpp-2 dynamic build emits DLLs the exe needs at runtime:
`C:\lt\release\{ggml,ggml-base,llama,llama-common}.dll` **plus** the backends dir
(`C:\lt\release\build\llama-cpp-sys-2-*\out\backends\` — ggml-vulkan.dll + per-CPU ggml-cpu-*.dll).
Dev builds find backends via the compile-time path; **bundled builds must ship them next to the exe**
(`load_backends_from_path(exe_dir)` fallback already implemented in `ai/local_llm.rs`).

## What to do next (priority order)

1. **Live end-to-end dictation test** (needs a human voice): per-stage logs now print pre-roll / capture /
   ASR / cleanup / inject ms. Verify pre-roll line, SendInput injection into a terminal, and local cleanup
   latency in the running app. Enable AI cleanup → Local (offline) in Settings → AI (model 769 MB download
   button there; already on disk for this machine).
2. **Live-test Apps panel**: add profile for e.g. "code", dictate into VS Code, check
   `app profile matched` log line.
3. **A5 pt-BR A/B on user's own voice** — the only remaining research blocker.
4. **B6 context-awareness** (UIA read of focused textbox before/selected/after → feed cleanup prompt) —
   next big differentiator, ~5-8d.
5. Consider flipping default `model_name` tiny → small (147 ms @ 11 s, best pt accuracy — measured headroom).
6. Product packaging: Tauri bundler + DLL shipping (see gotcha), model-download onboarding UX.

## Suggested skills (next session)

- `run` / `verify` — launch and drive the app for live verification
- `superpowers:finishing-a-development-branch` — branch has 6 feature commits; consider PR to master
- `commit-push-pr` — one-shot when ready
