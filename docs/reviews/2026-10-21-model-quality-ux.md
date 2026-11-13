# Model quality, cloud diagnostics and UX — 2026-10-21

Status: local development; installed Lectus remains untouched. This report separates inherited real-model evidence, newly reproduced HTTP defects, browser fixtures and work still requiring real speakers/devices. No cloud key was requested or used.

Subsequent evidence extends the initial corpus audit below: [public human corpus comparison](../benchmarks/2026-10-21-human-corpus.md) now records real Base/Turbo inference on2Mandarin,2US-English,2Brazilian-Portuguese and2MINDS pt-PT human clips. FLEURS adds independent read-speech references, synthetic noise and long stress. MINDS published references are Google ASR/MT, so PT-PT results are explicitly disagreement with those systems, not human-gold WER. The earlier "missing corpus" notes describe the initial repo inventory, not a claim that public evidence remained unavailable.

The later [two-model text translation comparison](../benchmarks/2026-10-21-translation-comparison.md) measured Qwen3 4B/Q4_K_M and official0.6B/Q8_0 on8new authored cases. The0.6B candidate showed omissions, wrong language and time corruption and was not integrated. The4B path preserved sampled numbers/negation but added unwanted prefixes in2cases and used Brazilian-Portuguese wording. Generation success is explicitly distinguished from translation adequacy; neither model is declared best.

## Recognition and translation evidence

| Engine | Task and available evidence | Limit |
| --- | --- | --- |
| Built-in Whisper Tiny/Base/Small/Large v3 Turbo | Real Windows Vulkan recognition of 3 existing EN/PT clips, recorded in [Windows benchmark](../benchmarks/2026-10-21-windows-models.md). | Two English clips and one Portuguese TTS clip; no verified European-Portuguese speakers, Chinese, accent coverage or human quality panel. The previous results are retained without repeating them merely to refresh a timestamp. |
| Built-in Qwen3 4B GGUF | Real local text translation, 5 short directions and two passes, in [translation benchmark](../benchmarks/2026-10-21-local-translation.md). | Text-to-text, no audio input. Warm RTX 3080 latency is not cold latency or a CPU prediction. No translation-quality ranking or PT-PT dialect guarantee. |
| Qwen3-ASR 0.6B/1.7B | Candidate confirmed in the publisher's current documentation: speech recognition and language identification, 30 languages and 22 Chinese dialects, Portuguese/English/Chinese/Cantonese listed. | Not integrated, downloaded or executed in Lectus. Publisher benchmarks are not this machine's results and do not establish performance for European Portuguese. |

Official sources checked on 2026-10-21:

- [OpenAI Whisper](https://github.com/openai/whisper): performance varies by language; Turbo is not trained for speech translation to English. Lectus can still use Turbo to recognize speech, followed by a separate text translation engine.
- [Qwen3-ASR publisher repository](https://github.com/QwenLM/Qwen3-ASR): the candidate has its own Transformers/vLLM inference toolkit. The existing Qwen3 GGUF text translator is not this ASR runtime.
- [Qwen3 4B GGUF publisher card](https://huggingface.co/Qwen/Qwen3-4B-GGUF): text generation model. Using it for translation requires separate task-quality evaluation.
- [whisper.cpp memory usage](https://github.com/ggml-org/whisper.cpp#memory-usage): upstream estimates are Tiny~273MB, Base~388MB, Small~852MB, Medium~2.1GB, Large~3.9GB. These are estimates for that runtime, not measured Lectus process/GPU peaks; Turbo is absent from this table. OpenAI's Python VRAM table must not be relabelled as Lectus RAM requirements.

## Initial corpus and subsequent public extension

The initial repo inventory in `src-tauri/tests/audio_samples` contained only `tts_en_4s.wav`, `jfk_en_11s.wav` and `tts_pt_6s.wav`. No private microphone recording was collected. Noise mixing/repetition of those clips tests synthetic acoustic and duration stress: repetition does not add speakers or establish regional accent quality. The subsequent [public extension](../benchmarks/2026-10-21-human-corpus.md) obtained8human clips under publisher CC-BY4.0 metadata, including Mandarin and a confirmed pt-PT dataset configuration; measured results, references, transformations and exact hashes are preserved there.

Use a provenance-bearing JSONL corpus with `id`, `task`, `source_url`, `license`, `file_sha256`, `language`, `locale`, `speaker_group`, `noise_type`, `snr_db`, `duration_seconds`, `reference`, `target_language`, `reference_translation`, and `synthetic_derivation`. Unknown locale stays unknown. Never label generic Portuguese as `pt-PT` by inference.

Measure recognition separately from translation:

1. ASR: report raw and normalized WER for PT/EN, CER for Chinese, plus number/date/name preservation. Retain normalization and exact edit counts. Report clean, noise and long-utterance groups separately.
2. Text translation: identical reference source text across engines, PT-PT/EN/ZH target-specific references, semantic omissions/additions, numbers/names and human native-speaker assessment. Automatic character/word scores alone cannot settle adequacy.
3. Runtime: model hash/backend/device/build hash, cold load, warm median/p95, real-time factor, process baseline/peak and measurement sampling interval; GPU memory measured separately. Run models serially to avoid shared-device distortion. Record unsupported/OOM as outcomes.
4. End-to-end dictation: record release-of-hotkey-to-insertion separately; model-only latency excludes recording, VAD, cleanup, translation and insertion.

[Google FLEURS](https://huggingface.co/datasets/google/fleurs) documents parallel speech and transcripts including Portuguese, English, Mandarin and Cantonese, with test speakers separated from training. The downloaded Portuguese subset is pt_br, explicitly Brazilian Portuguese. Separate MINDS14 pt-PT human recordings were obtained; their supplied transcripts are automatic, so a human-gold PT-PT accuracy evaluation remains outstanding. [FLORES](https://huggingface.co/datasets/facebook/flores) is a text translation benchmark and its current card points to FLORES+; it does not validate recognition. No new account or agreement was accepted.

## UX review and implemented cloud recovery

Impeccable was not found in the installed skill catalog or file searches under `.codex/plugins`, `.codex/skills`, `.agents/skills` and the project workspace. No Impeccable execution is claimed. The following is an independent source-level review; final visual/native acceptance remains separate.

- Existing native `details` help controls are keyboard accessible, and cloud permission distinguishes recorded audio from transcript text. Keep those boundaries.
- Existing `/models` connection check correctly says listing is not proof of inference permission. It is reused, not replaced with a second metadata test.
- Defects reproduced before correction: HTTP429 returned generic key/URL advice; failed transcription displayed a raw remote error body; model-list metadata could reflect the configured key into a displayed model ID. Three regression tests failed against the pre-fix source.
- `transcription/cloud.rs` now supplies safe recovery for rejected keys, denied account/model access, invalid endpoint/model, request size, quota/rate limit, service failure and redirects. Transport diagnostics identify timeout/connection/interruption without emitting raw reqwest errors, URLs or remote response bodies. Response reads are bounded to1MiB under existing request/connect timeouts. Invalid/control/oversized/key-reflecting model IDs are refused. Redirects remain disabled.
- `CloudSettings.tsx` retains stale-result revision protection, prevents duplicate same-render checks and redacts the configured key from unexpected IPC error/model-ID messages. The check's accessible description states that no audio/text is sent and that model listing does not prove inference access.

Native regression verification:10cloud tests passed,0failed,1live-provider test deliberately ignored on the benchmark-built native test executable, SHA256 `f7f458b262aea829a12831419ce2c6795f7101700ecbf8fe434672542d738abb`. This includes5new regressions, with3original defects first observed red before correction. The first green attempt encountered an expected in-progress integration symbol (`cancel_download` in another agent's module); it was not reported as a cloud pass. Final full-source suite and browser fixture receipts are coordinated by the parent session. No provider inference entitlement, account billing, TLS endpoint or real cloud request was tested.

The first [real recognition stress run](../benchmarks/2026-10-21-quality-stress.md) added synthetic10dB noise and60second repetition for EN/PT on Base/Turbo, with latency/RTF/process-memory measurements. The later [public human corpus run](../benchmarks/2026-10-21-human-corpus.md) added Mandarin/EN/PT-BR referenced recognition and pt-PT human audio against explicitly automatic publisher transcripts; broader accent coverage and native-speaker adjudication remain outstanding.

Independent download review found2issues and sent them to their owners: partial-state refresh hid actionable failure messages; selecting an existing final model through IPC lacked the new checksum gate. Source-level reinspection confirms error preservation/alerts and verification before selected-model loading are now present. Browser/native verification of those changes remains the coordinator's responsibility.
