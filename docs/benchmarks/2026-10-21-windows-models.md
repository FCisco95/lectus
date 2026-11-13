# Windows local speech model benchmark — 2026-10-21

Real Vulkan inference on an NVIDIA GeForce RTX 3080 using the existing `bench_latency` harness and already-downloaded models. No microphone, injection, cloud requests, settings changes or downloads. One warmup followed by three measured runs per clip, with and without Silero VAD. The benchmark passed in 42.07 seconds.

## Warm inference, VAD enabled

| Model | Load | English 5.4s, best | English 11s, best | Portuguese 7.6s, best | Portuguese observation |
|---|---:|---:|---:|---:|---|
| Tiny | 694 ms | 51 ms | 89 ms | 99 ms | Misheard “quinta-feira” as “quem está feira” |
| Base | 333 ms | 72 ms | 115 ms | 148 ms | Correct weekday; minor spacing |
| Small | 838 ms | 153 ms | 233 ms | 224 ms | Correct weekday; minor spacing |
| Large v3 Turbo | 2691 ms | 345 ms | 392 ms | 384 ms | Correct weekday; clean sentence |

Portuguese three-run timings with VAD: Tiny `[100,129,99]`, Base `[161,148,149]`, Small `[224,246,228]`, Turbo `[384,400,410]` ms. Without VAD, best Portuguese timings were 99, 132, 237 and 402 ms respectively.

## Decision and limits

Keep Base as the default for new installations: it correctly recognizes this Portuguese fixture with substantially less latency and memory. The user's installed Large v3 Turbo selection remains untouched; on this Windows machine it transcribes the fixture in under half a second after warmup, unlike the approximately four-second result in the [older M4 benchmark](2026-08-01-model-bench-m4.md).

Three short fixtures are a latency/regression sample, not a comprehensive accuracy ranking or end-to-end hotkey latency measurement. These numbers exclude recording, networking, cleanup and insertion. Cold model load and GPU warmup are separate costs. Medium and quantized Turbo variants were absent and skipped. Full Large v3 is newly selectable but was not downloaded or benchmarked.

Fixtures: `src-tauri/tests/audio_samples/tts_en_4s.wav`, `jfk_en_11s.wav`, `tts_pt_6s.wav`. Harness: `src-tauri/src/transcription/local.rs::tests::bench_latency`. Raw runtime log is `2026-10-21-windows-models.log` locally (ignored by Git); this summary preserves the measured results portably.
