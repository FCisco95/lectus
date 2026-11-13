# Recognition stress measurements — 2026-10-21

This is model inference on test audio, not live dictation or a clinical/human language evaluation. No microphone, app insertion, settings, credentials or installed executable are touched. No model or corpus download is performed.

This report covers the initial synthetic fixtures only. The subsequent [public human corpus comparison](2026-10-21-human-corpus.md) adds real Mandarin/English/Portuguese recordings and independent-reference measurements, with the automatic-reference limits of MINDS pt-PT stated explicitly.

Harness: `src-tauri/src/model_quality_test.rs`; runner: `scripts/benchmark-model-quality.cmd`. The runner launches a fresh native test process for Base and Large v3 Turbo, reading existing app-data models. `LECTUS_BENCH_MODEL_DIR` can point to another existing read-only cache. The isolated results go to `.context/quality-stress/<model>.json`; the decisive measurements will be preserved below after the run.

## Conditions

Each model receives four newly derived stress cases. Existing unmodified short-clip timings are already preserved in [the earlier Windows benchmark](2026-10-21-windows-models.md) and are not repeated here.

| Condition | Source | Derivation and coverage |
| --- | --- | --- |
| English noise | `tests/audio_samples/tts_en_4s.wav` | Deterministic uniform noise,10dB signal-to-noise ratio, fixed LCG seed; normalize the whole mix only if needed to avoid clipping. |
| English long | Same | Repeat/truncate the original samples to exactly60seconds. No extra speaker or spontaneous language. |
| Portuguese noise | `tests/audio_samples/tts_pt_6s.wav` | Same deterministic10dB noise procedure. Regional accent unknown; not labelled European Portuguese. |
| Portuguese long | Same | Same60second repetition; regional accent unknown. |

One warmup and three measured runs per case. Median warm latency and RTF exclude recording, VAD, translation, cleanup and insertion. VAD is deliberately off for identical engine input across models. Fresh-process model loading is timed separately; disk/driver/shader caches are not cleared. Windows working set and private commit are snapshots, while peak working set is the OS lifetime peak of that fresh process. This is not GPU memory, and no GPU memory figure is claimed.

## Accuracy boundary and external corpus support

No independently verified exact reference was found for the two TTS source files. Accuracy is therefore `null`, with an explicit reason, rather than treating another Whisper output as ground truth. Chinese, human PT-PT speakers, accents and real environmental recordings are absent. The experiment cannot rank models for those populations.

For independent public/consented recordings, set `LECTUS_BENCH_MANIFEST` to a JSON array file. Each object has `id`, `wav`, `language`, `locale`, `provenance` and optional `reference`. WAV paths are relative to that manifest; input must be PCM16 mono16kHz, nonempty and at most120seconds. Record source URL/license/authoritative transcript and accent information in `provenance`; use `unknown` when unverified. A supplied reference enables normalized WER for word-based languages or CER for Chinese/Cantonese. The report records edit count, denominator and normalization. Empty reference produces no rate. Unicode lowercase and punctuation-to-space normalization do not canonicalize Chinese scripts or establish translation quality.

Example (illustrative schema only; no such file is asserted present):

```json
[
  {"id":"pt-pt-speaker-01","wav":"public-fixture.wav","language":"pt","locale":"pt-PT","provenance":"Official source URL, license, speaker locale and reference-transcript provenance","reference":"Exact independently verified transcript."}
]
```

Unit checks cover substitution/insertion/deletion, empty input, Portuguese punctuation/diacritics, Chinese characters and deterministic10dB bounded noise. Both tests passed. Each real-model benchmark passed in its own process; the runner exited0. Native test executable SHA256: `f7f458b262aea829a12831419ce2c6795f7101700ecbf8fe434672542d738abb`. It is a test executable, not the application or a new installer; subsequent source changes require their own verification.

## Observed results

| Case | Base warm median | Base RTF | Turbo warm median | Turbo RTF |
| --- | ---: | ---: | ---: | ---: |
| English10dB noise,5.4s |52.9912ms|0.00981|181.0585ms|0.03353|
| English60s repetition |479.9090ms|0.00800|940.7308ms|0.01568|
| Portuguese10dB noise,7.62825s |126.0031ms|0.01652|222.1560ms|0.02912|
| Portuguese60s repetition |819.5838ms|0.01366|1210.2177ms|0.02017|

| Model | Fresh-process load | Baseline working set | OS lifetime peak working set | Final private commit |
| --- | ---: | ---: | ---: | ---: |
| Base |398.4639ms|10.26MiB|331.34MiB|1048.56MiB|
| Large v3 Turbo |1761.3457ms|10.27MiB|378.31MiB|2805.74MiB|

Vulkan was actually selected by both models. Working set is resident process memory; private commit is committed private virtual memory, not resident RAM or GPU VRAM. Their different sizes are expected; reporting just the working set as total model memory would be misleading. The test does not measure driver allocations or GPU VRAM. It establishes successful inference under these synthetic conditions, not language accuracy.

Measured warm runs in ascending milliseconds:

| Case | Base | Turbo |
| --- | --- | --- |
| English noise |52.8616,52.9912,53.5570|176.4774,181.0585,192.1814|
| English60s |410.5770,479.9090,515.5404|901.0425,940.7308,942.8267|
| Portuguese noise |116.2436,126.0031,141.1662|218.5970,222.1560,222.7738|
| Portuguese60s |813.4930,819.5838,826.0304|1207.1268,1210.2177,1212.0067|

Base output for noisy Portuguese contained “Rio Mio” and “da vez”; Turbo's output contained “reunião” and “10 horas”. These are observations about different generated outputs, not independently verified WER. No model is declared best. Long repetitive inputs are easier to predict than spontaneous long speech and cannot establish real-world60second dictation reliability.

Model SHA256: Base `60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe`; Turbo `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69`. Source WAV SHA256: English `feb5fe27f723f685522ad32071bfb97fe77024359bee45881f7ecdfd49dd83e8`; Portuguese `c5739105bb871c5229c979643491684fa816902f56e53c64e7522336eaf12776`.

Local JSON receipts retain per-case generated outputs, snapshots and provenance under `.context/quality-stress`; this portable report preserves hashes, all timings and the material limits. The runner now also includes the same two unit tests; those were executed directly on this exact test executable after the first runner invocation, with0failures.
