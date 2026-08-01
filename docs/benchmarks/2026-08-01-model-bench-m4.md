# Model bench — M4 Mac (Metal), 2026-08-01

Harness: `bench_latency` (1 warmup + best-of-3, RTF, full transcripts) and
`bench_cleanup` (1 warmup + best-of-3, TTFT via RUST_LOG) after the fair-A/B
upgrade (per-model chat templates, PT clip `tts_pt_6s.wav`). All model files
byte-verified against the Hugging Face API.

## Whisper (STT) — best-of-3 warm, Metal

| Model | Load | EN 5.4s clip | EN 11s clip | PT 7.6s clip | RTF | PT "quinta-feira" |
|---|---|---|---|---|---|---|
| tiny (was default) | 101 ms | 99–105 ms | 112–119 ms | 125–133 ms | 0.010–0.019 | **WRONG** ("quem está feira") |
| **base (new default)** | 105 ms | 168–178 ms | 213–236 ms | 247–248 ms | 0.019–0.033 | **correct** |
| small | 233 ms | 586–595 ms | 783–789 ms | 828–871 ms | 0.07–0.11 | correct |
| large-v3-turbo f16 | 898 ms | ~3.8 s | ~4.0 s | ~3.8 s | 0.36–0.76 | correct |
| large-v3-turbo q8_0 | 374 ms | ~3.6 s | ~3.7 s | ~3.7 s | 0.33–0.68 | correct |
| large-v3-turbo q5_0 | 246 ms | ~3.7 s | ~3.7 s | ~3.7 s | 0.34–0.71 | correct |

**Decision: default → `ggml-base.bin`.** Cheapest model with correct PT
(+ slightly better EN punctuation than tiny) at +60–120 ms per dictation —
well inside dictation tolerance. small = 6–8× tiny's latency with no quality
gain over base here. turbo variants are ~4 s/clip on the M4 — disqualified for
dictation on this hardware (quantized turbo was *slower* than f16 on Metal:
dequant overhead). Existing users keep their configured model; all rows remain
selectable in Settings → Models. **Re-bench turbo on the RTX 3080/Vulkan** —
the Windows machine may absorb it.

## Cleanup LLM — best-of-3 warm, Metal (Gemma template vs native templates)

| Model | EN filler-heavy | EN short | PT | TTFT warm | Quality |
|---|---|---|---|---|---|
| **gemma-3-1b Q4_K_M (kept)** | 366 ms | 279 ms | 605 ms | 78–165 ms | Best cleaner: fillers removed, punctuation/caps, PT "tá"→"está", no translation |
| Qwen3-1.7B Q4_K_M | 459 ms | 531 ms | 862 ms | 146–380 ms | Under-cleans (no caps/punct); slower. /no_think worked |
| Qwen3-0.6B Q4_K_M | 218 ms | 157 ms | 296 ms | 51–79 ms | ~2× faster but barely cleans — below incumbent |
| granite-4.0-1b Q4_K_M | 5.6–9.7 s | | | 263–485 ms | Broken on Metal (garbage "!!!" output + GGML_ASSERT teardown; arch supported, runtime bug) |

**Decision: keep Gemma 3 1B.** No candidate meets "latency ≤ AND quality ≥".
License note: Gemma Terms (not Apache) — revisit when a stronger Apache-2.0
~1B lands; the per-model template dispatch in `local_llm.rs` makes future A/Bs
one env var away (`LECTUS_CLEANUP_MODEL=file cargo test --release bench_cleanup -- --ignored --nocapture`).

llama-cpp-sys-2 0.1.151 vendors a llama.cpp with `qwen3` and `granitehybrid`
arch support — no crate upgrade needed or attempted.
