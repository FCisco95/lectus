# Lectus — working prompt (paste into a new session)

> **Goal:** local-first dictation with Handy's openness and Wispr Flow's best ideas (cleanup inside the
> latency budget, context-awareness), 100% offline, visually polished, and *actually measured*.
>
> Built from two completed research rounds (adversarially verified). Confidence is marked throughout.
> **Read §0 before proposing any work.**

---

## 0. THE ONE RULE

> ### ⚡ A1 MEASURED — 2026-07-15 (this section supersedes the priors below)
> Bench: `bench_latency` in `src-tauri/src/transcription/local.rs` (`cargo test --release bench_latency
> -- --ignored --nocapture`, Vulkan build env). RTX 3080 = Vulkan0, batch-1, greedy, auto-detect,
> 1 warmup + 3 timed, best-of-3:
>
> | Model | Load | 5.4s EN clip | 11.0s EN clip | RTF |
> |---|---|---|---|---|
> | ggml-tiny | 414 ms | **39 ms** | **55 ms** | 0.005–0.007 |
> | ggml-base | 190 ms | **49 ms** | **77 ms** | 0.007–0.009 |
> | ggml-small | 491 ms | **124 ms** | **147 ms** | 0.013–0.023 |
>
> Transcripts correct at all sizes. **ASR is 25–50x under the 200–400ms target → the ">5s" claim is dead,
> and per §0's own rule most of the plan evaporates:**
> - **A2 (CUDA A/B): CANCELLED.** A 3.2x win would save ~35ms. Not worth 2-4 days + ship risk.
> - **A4 (Parakeet): latency motivation gone.** Only surviving rationale = fallback engine. Deprioritized.
> - **Model picker can default to `small`** — 147ms @ 11s buys the best pt-BR accuracy available.
> - Any *perceived* slowness lives in: capture tail, **cloud cleanup**, injection, or UI transitions.
>   Cleanup + injection timing logs added to `lib.rs` same day (`pipeline: AI cleanup/injection took N ms`).
> - Still unmeasured: pt-BR on own voice (A5), live end-to-end per-stage split.
>
> **Surviving priority order: B1 (pre-roll) · A3 (SendInput) · B2 (local cleanup LLM) · A5 (pt-BR A/B) · A6/A7.**

**Every latency number below this box is a prior, not a measurement.** No source anywhere measured
whisper.cpp Vulkan-vs-CUDA on Ampere at dictation length. No source measured batch-1 short-utterance latency
on an RTX 3080 for *any* engine. The "~200ms cleanup budget" is unvalidated by anything.

Three independent verifiers reached the same conclusion: **RTX 3080 numbers must be measured locally, not cited.**

The claim "Lectus transcription takes >5s" is **unverified** and predates Vulkan, GPU warmup, and the timing
logs. Nobody has read the current number. **Measure first (task A1). If it's already ~1s, most of this plan
evaporates and the work is polish instead.**

---

## 1. Ground truth — verified at commit `e9c94bf` (2026-07-15)

Tauri 2 + Rust + React/TS. Windows 11. **NVIDIA RTX 3080 (Ampere, 10GB VRAM).**

Pipeline: hotkey (`Ctrl+Shift+Space`, hold-to-talk + toggle, two-key combos) → cpal capture (mic picker)
→ whisper.cpp via **`whisper-rs` 0.16 (direct dependency)** → optional cleanup (Claude CLI or Groq **cloud**)
→ inject into focused field.

### ALREADY DONE — never propose these as new work
`whisper-rs` features `["openmp","vulkan"]` · `use_gpu(true)` · Vulkan running (RTX 3080 = `Vulkan0`) ·
`WhisperState` pre-created at load · GPU warmup at load · model+state load ~0.5s · AVX2/FMA ·
pipeline timing logs · ggml tiny/base/small picker · multilingual auto-detect · custom dictionary · history ·
pill + draggable orb UI · mic picker

Vulkan build recipe: memory `lectus-vulkan-build.md` — needs `VULKAN_SDK`, `CMAKE_GENERATOR=Ninja`,
`CARGO_TARGET_DIR=C:\lt` (junction → `src-tauri\target`, dodges MAX_PATH), `PATH` set **before** `call vcvars64.bat`.

### Lectus already has a VAD — do not plan it as greenfield
`src-tauri/src/audio/vad.rs` (88 lines, `EnergyVad`, RMS threshold, `silence_frames_required: 30`, unit tests).
`src-tauri/src/audio/mod.rs:2-4` states it is *"no longer on the dictation path (Phase 2.5 stops on key-release,
not VAD silence)"*. Only live call site: `src-tauri/src/lib.rs:403` → `audio::vad::rms(&chunk)` driving the level
meter. This was a **deliberate decision tied to push-to-talk**, per `docs/superpowers/plans/2026-06-01-lectus-phase2.5.md:755`.

### Hard constraints
- **English + Portuguese (pt-BR).** Disqualifies English-only models. **This is the binding constraint** — it is
  why the backend choice is genuinely hard, not a preference.
- **Zero network** in the hot path. The Groq/Claude-CLI cleanup is a cloud dependency on the chopping block.
- **10GB VRAM holds ASR + cleanup LLM.** (Spoiler: VRAM is *not* the constraint — see §4.)
- Target: ~200-400ms ASR + ~200ms cleanup. **Both numbers are hypotheses.**

---

## 2. Settled facts — do not re-research

### Wispr Flow (round 1, verified)
- **100% cloud.** Own docs: *"Transcription always occurs on the cloud"*; no on-prem at any tier; fails to
  activate offline. **There is no local Wispr architecture to clone. Offline is Lectus's differentiator** —
  but cloud is *how* Wispr buys its latency, so offline is simultaneously the binding constraint.
- **Deliberately non-streaming.** Waits for end-of-utterance → fine-tuned Llama cleanup → emits.
  Their words: *"Streaming might give your writing speed, but Flow gives it clarity and meaning."*
  ~700ms p99 budget (<200ms ASR + <200ms LLM + <=200ms network) — a **stated target on datacenter GPUs**,
  from Wispr-affiliated sources only. **Never a benchmark to chase.**
- **Cleanup belongs inside the budget.** Lectus treating it as an optional post-pass is architecturally backwards.
- **Context-awareness is a model input, not UI.** Payload (confirmed in their public API schema): app identity,
  textbox `before_text` / `selected_text` / `after_text`, on-screen text, coding variable/file names,
  session app list, optional screenshot. Accessibility-text read default ON; Screen OCR opt-in, gated behind it.
- **Unresolved:** which ASR model Wispr uses. Both "trains its own" and "uses Whisper-family" were refuted 0-3.
  Do not assert either.

### Handy (round 2, heavily verified — most headline claims were REFUTED)
- **REFUTED: "Handy runs ASR on CPU."** Its `src-tauri/Cargo.toml` (v0.9.3) declares, under
  `cfg(all(windows, target_arch="x86_64"))`: `transcribe-cpp = { version = "0.1.3", default-features = false,
  features = ["dynamic-backends","vulkan"] }`. **That is parity with Lectus, not a gap.** Killed by three
  independent verifiers.
- **Handy ships two ASR crates, no `whisper-rs` at all** (it migrated off): `transcribe-rs 0.3.8` with
  `features = ["onnx"]` (Parakeet, Moonshine, SenseVoice, GigaAM, Canary, Cohere) **plus** `transcribe-cpp` for
  Whisper GGUF/ggml. macOS `["metal"]`; Windows ARM64 CPU-only.
- **The model-coverage gap is bigger than assumed.** Handy lists **15 models across ~7-8 engine families**.
  **Lectus matches ~1/7 to 1/8**, not half.
- **Handy ships ONNX Runtime CPU-only on Windows and deliberately REMOVED the DirectML EP** — its Cargo.toml
  comment cites pyke's prebuilt ORT compiling with a global `/arch:AVX2` baseline that crashes at process startup
  on pre-Haswell CPUs. So **Parakeet/ONNX on Windows does run on CPU** — that part is real. *A listed EP flag
  being available does not mean it is shippable; the reference app removed one.*
- **REFUTED: "Handy's GPU path is unreliable — a stability gap Lectus can beat."** Issue **#91** is an **AVX2 CPU**
  fault, not GPU. The GPU crashes are **closed**: **#209** (RTX 3050 → PR #212), **#1137** ("Vulkan backend GPU
  selection crash"). v0.9.2 (2026-07-12) shipped *"better GPU detection and selection"*.
  **The inversion that matters: #1137 was a *Vulkan* crash, and Lectus runs the same whisper.cpp+Vulkan engine
  family. Lectus INHERITS this bug class — it does not beat it. On this axis Handy is ahead** (it has a CPU-only
  Parakeet fallback as its documented *"most stable model"*; Lectus has no non-whisper.cpp fallback at all).
- **Handy's one real lead is temporary.** Its *recommended* model (Parakeet V3) has no shipped GPU path — but
  maintainer cjpais (Discussion #494, 2026-03-02): *"It's being worked on officially, the main issue is
  distribution and ci/cd pipelines."* **A ~1-release lead, not a moat.**
- **Handy uses Silero neural VAD to gate frames pre-inference**: `vad-rs` (a **cjpais fork**, not the crates.io
  crate), `silero_vad_v4.onnx` from `blob.handy.computer`, **30ms frames / 960 samples @16kHz**, LSTM state
  carried across frames with per-session reset. `audio_toolkit/audio/recorder.rs` drops `VadFrame::Noise` before
  the buffer. `VadPolicy` = `Disabled` / `Offline` / `Streaming`.
- **Handy's open bugs Lectus can beat (UNVERIFIED — no verifier checked this list):** **#502** *"Pastes clipboard
  instead of spoken text"* (critical) · **#1213** Windows 11 stuck on "transcribing" after sleep · **#439** Direct
  paste ignores keyboard layout · **#429** first character dropped · **#1682** Windows-on-ARM GPU failure.
- **Do not reason about Handy's user base.** "Handy's biggest platform is Windows" was flagged **fabricated** by
  two verifiers — no telemetry or per-OS split exists anywhere.
- **Forward risk:** `handy-computer/transcribe.cpp` (*"ggml STT for 16+ model families"*, 60+ variants incl.
  Parakeet ×10, + CUDA/Vulkan/Metal EPs) landed in Handy **v0.9.0**, which also **added streaming model support**.
  This may supersede the ONNX path and bring GPU Parakeet generically.

### Windows injection — round 1's open question is now RESOLVED
**`SendInput` is the field consensus.** Three independent implementations converge: `xarthurx/whisperi`
(**Tauri 2.x + Rust — your exact stack**), `drajb/whisper-local`, `bhargavchippada/faster-whisper-dictation`.
Whisperi uses native Win32 `SendInput` *"which means it can paste directly into command-line interfaces"* —
**terminals and CLIs, where clipboard injection fails.**

Round 1 ruled out the alternatives: UIA `TextPattern` is **read-only by Microsoft's design** (both .NET and current
Win32 COM surfaces); TSF needs a registry-registered in-proc COM DLL **the user must select as an input method**;
`ValuePattern` is a trap (`SetValue` replaces the whole value instead of inserting at the caret, and multi-line
edits don't support it — it fails exactly where dictation lives). **Use UIA to READ context, `SendInput` to WRITE.**

---

## 3. The backend decision — evidence is thin; DO NOT MIGRATE

**Does Vulkan leave speed on the table vs CUDA on Ampere? Probably, magnitude unknown.** Three inference chains
point the same way and each is broken differently:

| Prior | Number | Why it's broken |
|---|---|---|
| llama.cpp **#17273** | CUDA **1.50x** prefill (4462 vs 2972 t/s), **1.30x** decode (150.7 vs 115.7 t/s) | **A100**, **LLM** workload, `bug-unconfirmed`+`stale`, no maintainer reply, **never verified by anyone**. Whisper's encoder is prefill-shaped → the 1.50x is the relevant one, but that's extrapolation. |
| Handy **#494** | **3.2x** (61s → ~19s), GPU util **55% → 97%** | **RTX 4060 Ti = Ada, not Ampere**; **190s clip = long-form**; unshipped community build; **journal never establishes whether it measured Whisper or Parakeet** (thread titled "Parakeet", repro flips the *whisper* feature). |
| whisper.cpp **#2375** | *"Vulkan is as fast as CUDA"* | **Unquantified user impression. No CUDA baseline in the thread at all.** Sole NVIDIA datapoint: RTX 3060, long-form, RTF ~0.033. Vulkan's real claimed edge is **hardware breadth, not peak speed on NVIDIA.** |

**REFUTED three times (verdicts 83, 90, 105): "CUDA is a one-line Cargo.toml change."** `whisper-rs 0.16` does
expose `cuda` (→ `whisper-rs-sys ^0.15`), and **default features = none (0 of 12)** — a build omitting them
silently falls back to CPU. But #494 documents the real procedure: `CMAKE_GENERATOR=Ninja`,
`CUDAFLAGS="-allow-unsupported-compiler --std=c++17"`, `CUDAARCHS="86"` (Ampere), `LIBCLANG_PATH`, **six distinct
error classes**, 10-15 min first build. **Verifier's corrected costing, verbatim: "2-4 engineer-days with real
CI/distribution risk, not a flag flip."**

**Shipping CUDA ≠ measuring it.** Ships need CUDA Toolkit 12.0+ at build; bundling `ggml-cuda.dll` +
`cudart64_12.dll` + `cublas64_12.dll` (**~100s of MB**); a **separate NVIDIA-only build matrix** because
whisper.cpp GPU backends are **compile-time-only with no runtime fallback**. Vulkan needs **zero redistributables**
(the loader ships with the driver) — that is why it's everyone's ship target, including Handy's.
**Trap: whisper.cpp #2857 — `CUBLAS=0`, a nominal "CUDA build" silently running CPU. Verify GPU residency, not
just wall-clock, or your A/B is invalid.**

**Parakeet TDT v3 — packaging objection DEAD, speed unproven, control surface REGRESSES:**
- ✅ **Windows/Rust path is well-trodden** (verdict 150, refuting the objection): pre-exported int8 ONNX
  (`csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8` ~640MB, or `istupakov/parakeet-tdt-0.6b-v3-onnx`) +
  sherpa-onnx Rust API + CUDA/DirectML EP. **Zero vendor export work.** The decisive point: *the benchmark app
  being torn down already ships that exact path in production.*
- ⚠️ **UNRECONCILED CONTRADICTION:** the primary extraction says `sherpa-rs` is **deprecated** (*"upstream
  sherpa-onnx now provides an official Rust API, so this repository is no longer maintained"*), yet verdict 150
  recommends `sherpa-rs`. **No verifier reconciled these. Prefer the official upstream `sherpa-onnx` Rust API.**
- ⚠️ **Speed unproven on your hardware.** RTFx **3,332.74** is **A100-80GB @ batch-64** (arXiv 2510.06961, which
  explicitly disclaims transfer to other hardware). Strongest surviving argument is same-harness architectural:
  **Parakeet 3332.74 vs Whisper large-v3 68.56 RTFx (~48x)**. Consumer figures (parakeet.cpp ~27ms/10s vs
  whisper.cpp 150-500ms/10s, RTX 3090-class) were **flagged blog-tier by the verifier who cited them**.
- ⚠️ **Portuguese: likely fine, formally unmeasured.** pt WER **Fleurs 4.76 / MLS 7.50 / CoVoST 3.96** — strong
  tier, better than its own English. **But** computed *after removing punctuation and capitalization* (for a
  dictation app, punctuation **is** the deliverable — this is an argument **for** the cleanup stage), and NVIDIA
  self-discloses **European-Portuguese training vs Brazilian-Portuguese benchmarks**. Verdict 135: *"not safely
  predicted in either direction — an evidence gap, not a known penalty."* Noise: **38.11% WER at SNR 5dB.**
- 🚩 **CONTROL-SURFACE REGRESSION — the strongest argument for keeping Whisper.** Parakeet v3 has **no
  language-forcing/hint parameter** (NeMo **#14799**, **#15097**), reportedly **defaults to English on non-English
  speech**; Handy's docs confirm *"Language is detected automatically and cannot be manually specified."*
  Whisper gives **99+ languages plus manual override**. **For EN/PT code-switching at 3-15s, this is a downgrade.**
- 🚩 Counter-argument preserved (never independently verified): whisper-large-v3-turbo is **809M and ~6x faster**
  than large-v3; two sources argue Parakeet's real-world edge is *"marginal and infrastructure-dependent."*

### Recommendation
**Do not migrate.** (a) **Measure CUDA-vs-Vulkan locally** on your own short utterances — highest-information
experiment available, and verdict 105 explicitly steelmans this narrow version as *"defensible."*
(b) **Add Parakeet as a second, selectable engine** — which also buys the non-whisper.cpp fallback the verifiers
say you lack versus Handy. (c) **Promote it to default only after a pt-BR A/B on your own voice** resolves both
the accuracy question and the language-forcing regression. Anything stronger is unsupported by the evidence.

---

## 4. GPU contention — VRAM answered, contention NOT

- ✅ **VRAM is a non-issue.** `unsloth/gemma-3-270m-it-GGUF`: **Q4_K_M 253MB, Q8_0 292MB, F16 543MB** — **under
  1GB of 10GB even at full precision**, co-resident with a ggml small/base Whisper with room to spare. Parakeet's
  side is equally cheap (600M params, ≥2GB to load). **On a 10GB card, VRAM does not constrain you.**
  Gemma 3 270M covers 140+ languages with Portuguese present → not disqualified. It's GGUF → drops into the same
  llama.cpp/ggml runtime `whisper-rs` already links.
- ✅ **Runtime choice is forced.** **vLLM disqualified**: `gpu_memory_utilization` defaults to **0.90**,
  pre-allocating ~90% of VRAM at startup (*"not a bug, it's the design"*) — starves a resident whisper context;
  locks one model per server instance; and buys nothing for one user (**RTX 4090, Llama 3.1 8B: vLLM ~71 vs
  llama.cpp ~65 vs Ollama ~62 tok/s**). **Ollama** costs **5-15% (up to ~30%)** over direct llama.cpp and is a Go
  daemon on `localhost:11434` needing process management. **→ Embed llama.cpp in-process.**
- ❌ **Serialization is NOT ESTABLISHED. The journal has nothing.** No measurement of SM contention between a
  resident whisper context and a llama.cpp context, no context-switch cost, no evidence on whether a Vulkan ASR
  context and a CUDA LLM context coexist cleanly under one driver. **Open question — measure it.**
- ❌ **The ~200ms cleanup budget is unvalidated.** No TTFT for Gemma-3-270M / Qwen3-0.6B / Llama-3.2-1B on any
  GPU. The search agent's own words: the small-model end of the curve is *"essentially unbenchmarked publicly."*

---

## 5. Ranked plan

> Effort: **only A2's "2-4 days" is evidence-sourced.** All other estimates are judgment, marked ⚠.
> Nothing from ALREADY DONE appears here.

### Tier A — beats Handy

| # | Change | Delta | Effort | Risk |
|---|---|---|---|---|
| **A1** | **MEASURE FIRST.** Instrument short-utterance (3-15s) wall-clock on your own **EN + pt-BR** voice, batch-1. Read the existing timing logs. Report per-stage: capture → ASR → cleanup → inject. | Zero latency delta; **unblocks every decision below** | ⚠ 1-2 d | **None. Not doing it is the risk.** |
| **A2** | **CUDA vs Vulkan A/B — LOCAL ONLY, do not ship.** `whisper-rs = { features = ["openmp","cuda"] }`. **Verify GPU residency** (#2857 `CUBLAS=0` silent-CPU trap). Expect the #494 gauntlet: Ninja, `CUDAFLAGS`, `CUDAARCHS="86"`, `LIBCLANG_PATH`. | **Unknown — that's the point.** Priors 1.30-1.50x (wrong workload) to 3.2x (wrong GPU + long-form). | **2-4 d** *(sourced)* | **Med-High.** Shipping needs Toolkit 12.0+, ~100s-MB DLL bundle, separate NVIDIA-only matrix. **Keep local until the number justifies it.** |
| **A3** | **`SendInput` injection path.** Field consensus, 3 independent impls, one on **Tauri 2 + Rust**. Unlocks terminals/CLIs where clipboard fails. **Directly beats Handy #502 / #439 / #429.** | Correctness, not latency. **Clearest polish win available.** | ⚠ 3-5 d | **Med.** Keyboard-layout handling is exactly what Handy's Direct path got wrong (#439). Keep clipboard as fallback **with the thread-lock save/restore pattern** — the naive version races the target app's async clipboard read (that IS #502). |
| **A4** | **Parakeet TDT v3 as a SECOND selectable engine** via official **sherpa-onnx Rust API** + pre-exported int8 ONNX + `cuda` EP. Use `static`/`download-binaries` to avoid loose DLLs. **MANDATORY: `SetDllDirectory`/`AddDllDirectory` shim at init** — sherpa-onnx **#3059**: System32's `onnxruntime.dll` hijacks the bundled one (stale ORT 1.17.1 exposes only API 1/17; modern models need v23 → hard crash on end-user machines). | Latency **unmeasured on 3080 batch-1**. Buys the **non-whisper.cpp fallback you lack vs Handy**. | ⚠ 4-7 d | **Med-High.** #3059 DLL hijack; sherpa-onnx builds **MT/static-CRT by default, no switch**, conflicting with MD-mode Rust/Tauri; `ort-cuda` adds **~800MB+**. **Beats Handy only while its CUDA CI stays blocked — cjpais says it's being fixed.** |
| **A5** | **pt-BR A/B: Parakeet v3 vs whisper base/small, your own voice.** Every Portuguese verdict demanded this. **Test language-forcing and EN/PT code-switching explicitly.** | Resolves the **only** blocker on promoting Parakeet | ⚠ 1-2 d | **Low cost, high value.** If auto-detect misfires on pt-BR, A4 stays secondary permanently. |
| **A6** | **Silero neural VAD, frame-gating pre-inference.** **NOT greenfield** — replace `EnergyVad`'s RMS with Silero quality, and gate frames *before* the encoder. Copyable params: **threshold 0.6, 200ms trailing silence, 250ms min speech, 90s max**. Handy's: 30ms/960-sample frames, LSTM state across frames, reset per session. | **Unquantified.** Mechanism sound (less audio → less encoder work) + attacks Whisper's *"hallucinations especially after silence or noise"*. | ⚠ 2-4 d | **Low.** Hold-to-talk already endpoints on key-release → **endpointing value ≈ zero. The win is trimming + hallucination suppression.** |
| **A7** | **Hallucination hardening** with knobs you already have: `--context 0`/`64`, `--entropy-thold 2.6`, `--logprob-thold -1.25`, `condition_on_previous_text=false`, `suppress_tokens`. (`raw-api` is whisper-rs's escape hatch, no extra deps.) | Robustness. No numbers in evidence. | ⚠ 1-2 d | **Low but it directly trades against your dictionary.** whisper.cpp #2286: *"Once a single one of these gets into your prompt history, it tends to affect your next output"* — mitigation is `--context 0`, **which disables the mechanism vocabulary biasing depends on. Measure both.** |

### Tier B — nice to have

| # | Change | Delta | Effort | Risk |
|---|---|---|---|---|
| **B1** | **500ms pre-roll ring buffer** (`whisper-local`: *"captures the 500ms before you press the key... so the first word is never clipped"*). Warmup is done; **pre-roll is not.** | Perceived-instant capture, no clipped first word | ⚠ 1-2 d | **Very low. Best effort-to-perceived-quality ratio in the report.** |
| **B2** | **Measure the cleanup model before choosing it.** `unsloth/gemma-3-270m-it-GGUF` Q4_K_M (253MB), llama.cpp **in-process**. **Override the card's prescribed temp 1.0 / top_k 64 / top_p 0.95 toward near-greedy** for deterministic cleanup. | **Validates or kills the 200ms budget.** | ⚠ 2-3 d | **Med.** A 0.3B model's instruction-following is the open question. |
| **B3** | **Then: constrained scoring, not free-form generation.** Prompted **Llama-3.2-1B = 0.566** macro F1 vs **scorer 0.893 (no FT) / 0.937 (LoRA)**; **ELECTRA-Small 0.913**. Non-autoregressive, word-boundary decisions, K=2 lookahead, **input preserved verbatim → structurally eliminates "model answers the dictation instead of cleaning it."** | Correctness of cleanup | ⚠ 5-8 d | **High.** **Latency unmeasured (paper defers it); English-only (IWSLT 2017), nothing for Portuguese. Do not start before B2.** |
| **B4** | **Per-app profiles** (VoiceInk "Power Mode" equivalent): frontmost app → auto-swap prompt + model + settings. Windows: **`GetForegroundWindow` + `GetWindowThreadProcessId` + `QueryFullProcessImageName`** via the `windows` crate. **VoiceInk is GPL-3.0 + macOS-only — ideas only, no source lifting.** | Field's flagship differentiator | ⚠ 4-6 d | **Low technically; scope creep is the risk.** |
| **B5** | **Re-register global hotkey on session change** — documented Tauri 2 defect: *"Global hotkey may stop working after a remote desktop session."* | Reliability | ⚠ 0.5 d | **Very low. Nearly free.** |
| **B6** | **Context-awareness, local** (the Wispr idea worth stealing): read focused app + textbox before/selected/after cursor via UIA (`uiautomation-rs`/`windows-rs`), feed to cleanup LLM and/or whisper `initial_prompt`. Windows needs **no explicit a11y permission** (unlike macOS) — lower bar. | Wispr's core differentiator, offline | ⚠ 5-8 d | **Med.** Wispr's tri-partite cursor split is the shape to copy; the **injection mechanism into inference is undisclosed** — pick one and measure. Interacts with A7's `--context 0` tension. |
| **B7** | **Voice commands + deterministic offline formatting + "scratch that"** (model on `whisper-local`'s `commands.yaml`). | Parity with closest Windows-first competitor | ⚠ 5-8 d | **Low-Med. Shell execution from voice is a security surface** — Handy ships *"prompt injection defense"* for a reason. |

### Explicitly NOT recommended
- **LoRA fine-tuning for Portuguese.** The evidence argues **against**: fine-tuning Whisper on **~120h / 17,000
  samples scored WORSE than the untouched baseline at every size** (medium **31.37% FT vs 27.82% baseline**;
  tiny 41.89% vs 40.27%), while **decode-time biasing beat both at 9.51% (+18.31pp)**. LoRA-Whisper's real PT gain
  (13.34% → 10.81%) costs **160 hours of Portuguese audio**, is **Whisper-small only**, and **there is zero
  evidence LoRA adapters survive ggml quantization** — the load-bearing question for your pipeline, unanswered.
- **sherpa-onnx hotword boosting on the current stack.** *"Only transducer models support hotwords"* — Whisper
  gets nothing. Requires **abandoning greedy decoding for `modified_beam_search`** (latency cost) + shipping
  modeling-unit-matched bpe-vocab artifacts. **A consequence of A4, not an independent item.**
- **NeMo word boosting** (PR #14277 / NeMo 2.5.0). Lives in the **Python NeMo stack**; **non-NeMo runtimes (your
  ONNX/sherpa path) won't have it unless the export carries it.** Community-attributed, date unconfirmed.
- **Streaming partial results.** Field evidence: **~1.5s behind real-time**, and the implementing project
  **recommends batch mode as most accurate**. Wispr is non-streaming by choice.
- **vLLM / Ollama / `dillondesilva/tauri-local-lm`** (the last is Python-driven out-of-process, 3 commits, no
  releases — contributes nothing).

---

## 6. Evidence gaps — do not paper over these

1. **No whisper.cpp Vulkan-vs-CUDA measurement on Ampere exists** at any audio length. The whisper-specific
   thread was fetched and found empty on this. **The A100 llama.cpp figures were never adversarially verified.**
2. **No RTX 3080 batch-1 short-utterance latency for ANY engine.** Every latency claim here is a prior.
3. **The ~200ms cleanup budget is completely unvalidated.** Angle 4 produced **zero verification verdicts**.
4. **GPU *serialization* entirely unaddressed** (VRAM is answered; contention is not).
5. **Verification was uneven. All 35 verdicts landed on Handy + Parakeet/STT.** The CUDA, cleanup,
   personalization, and field-survey angles got **extracted claims only, zero adversarial checking** — and the
   verified angles saw **~2/3 of their load-bearing conclusions refuted or corrected**. **Assume a comparable
   error rate in everything unverified: the llama.cpp numbers, all LoRA figures, the TCPGen WERs, the punctuation
   F1s, the Gemma GGUF sizes, all vLLM/Ollama figures, Handy's open-issue list, every field-survey claim.**
6. **pt-BR is unmeasured and must not be guessed.** No independent reproduction of Parakeet's Portuguese figures
   was found by any agent (corroboration is family-level only).
7. **Punctuation quality for Portuguese is unevidenced for every candidate.** Parakeet's WERs strip punctuation
   before scoring; the punctuation paper is English-only.
8. **`sherpa-rs` deprecated vs verdict-150 recommending it — unreconciled.**
9. **Handy #494's 3.2x is of ambiguous provenance** (Whisper or Parakeet? thread title and repro disagree).
10. **Marketing noise to keep out of decisions:** VoiceInk's *"99% accuracy, almost instantly"*; Handy's
    *"95% accuracy on noisy inputs"* / *"cut processing time 40%"* (verifier: *"unsourced SEO fluff, must NOT be
    carried into the report"*); Northflank's sweep (vendor selling GPU hosting).

---

## 7. Working rules
- Small, focused diffs. **Ask before changing the capture/injection or shortcut pipeline.**
- `tauri-plugin-global-shortcut`/muda **cannot** register a bare modifier — it panics. Hotkey stays `Ctrl+Shift+Space`.
- API keys in env/config only — never in chat, never committed.
- Kill `chirp.exe` before every build; the running exe locks the binary.
- Internal crate may still read `chirp`/`chirp_lib` (repo renamed from `chirp`).
- **Evidence before assertions. Measure, then claim.**
