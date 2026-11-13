# Public human speech comparison — 2026-10-21

Real local ASR inference completed on eight public human recordings: two each of Mandarin Chinese, US English, Brazilian Portuguese and the publisher's European-Portuguese configuration. Base and Large v3 Turbo used existing model caches; no account, agreement, cloud inference, microphone, insertion, installed binary or model download was used. The experiment is small and cannot establish a universally best model.

## Corpus, attribution and provenance

Six clips are from [Google FLEURS](https://huggingface.co/datasets/google/fleurs), Conneau et al.,2022, [FLEURS paper](https://arxiv.org/abs/2205.12446), licensed [CC-BY4.0](https://creativecommons.org/licenses/by/4.0/). They are public human read speech, from `validation`, configs `cmn_hans_cn`, `en_us`, `pt_br`, rows0–1. FLEURS is intended for read-speech evaluation; performance need not match spontaneous noisy dictation. `pt_br` is Brazilian Portuguese, explicitly not PT-PT. No individual regional-accent claim is inferred from a generic language label.

The test-split viewer returned500; its first-rows endpoint reported a parquet scan exceeding its300MB limit. This was a technical viewer limit, not an access gate. The smaller validation split returned200. Only six individual WAVs were transferred, totalling4,809,308bytes; no full parquet/archive was downloaded. Cached asset revision was `70bb2e84b976b7e960aa89f1c648e09c59f894dd`.

Two further clips are from [PolyAI MINDS14](https://huggingface.co/datasets/PolyAI/minds14), Gerz et al.,2021, [original paper](https://arxiv.org/html/2104.08524), also labelled CC-BY4.0 by its publisher. Config `pt-PT`, train rows0–1, paths `pt-PT~CASH_DEPOSIT/response_4.wav` and `pt-PT~CASH_DEPOSIT/response_17.wav`; response metadata maps lang_id11 to pt-PT. These add110,028 transferred bytes, for4,919,336 total. The original paper describes human regional-native collection, but its §4 explicitly says written transcripts are Google ASR hypotheses and English versions are machine translations. Thus the comparison against those texts is disagreement with an independent ASR system, not a human-gold PT-PT accuracy measurement.

Audio conversion used existing ffmpeg: FLEURS float32 mono16k WAV to PCM16; MINDS phone-band audio to mono16k PCM16. Conversion cannot recover frequencies absent from the original. Six deterministic10dB uniform-noise derivatives and three approximately60second complete-clip concatenations were generated from FLEURS, retaining the corresponding independent references; these are synthetic stress, not new speakers or real room noise. Hashes below identify all clean original/converted files; derived hashes and exact ordered references are in the local manifest.

Canonical metadata can be recovered without signed asset URLs using `https://datasets-server.huggingface.co/rows?dataset=google%2Ffleurs&config=<config>&split=validation&offset=<row>&length=1` and the analogous `dataset=PolyAI%2Fminds14&config=pt-PT&split=train`. No signed URLs or expiry tokens are included in this report.

## Recognition accuracy and latency

One warmup and three timed runs per case, VAD off, forced spoken language, no cleanup or translation. Each model ran in a fresh process. Micro error rates below are total edit distance divided by total reference units in each condition; PT/EN use words and ZH uses characters. Normalization lowercases Unicode, replaces punctuation with spaces and removes Chinese whitespace. It does not map Traditional to Simplified Chinese, verbalized numbers to digits, or synonyms. Consequently script/formatting differences can count as errors. All reference transcripts came from dataset metadata, never the tested model's output.

| Corpus group | Condition | Base edits/units (rate) | Turbo edits/units (rate) |
| --- | --- | ---: | ---: |
| Mandarin CER | clean | 33/79 (41.77%) | 1/79 (1.27%) |
| Mandarin CER | noise | 36/79 (45.57%) | 2/79 (2.53%) |
| Mandarin CER | long | 60/193 (31.09%) | 0/193 (0.00%) |
| US English WER | clean | 5/48 (10.42%) | 5/48 (10.42%) |
| US English WER | noise | 18/48 (37.50%) | 5/48 (10.42%) |
| US English WER | long | 10/126 (7.94%) | 8/126 (6.35%) |
| Brazilian Portuguese WER | clean | 14/49 (28.57%) | 4/49 (8.16%) |
| Brazilian Portuguese WER | noise | 20/49 (40.82%) | 4/49 (8.16%) |
| Brazilian Portuguese WER | long | 24/98 (24.49%) | 8/98 (8.16%) |

These rates describe two source recordings per language. Repeating clips does not increase independent sample size. Base's Chinese results contain script/recognition differences; no script harmonization was performed. Turbo showed fewer differences on the sampled Chinese/PT-BR recordings, while neither changed its five English clean-reference edits. This is evidence for these cases, not a language-wide ranking.

| Case | Seconds | Base median ms / RTF | Turbo median ms / RTF |
| --- | ---: | ---: | ---: |
| cmn_hans_cn-validation-0-id1579 | 18.38 | 172.05 / 0.0094 | 320.89 / 0.0175 |
| cmn_hans_cn-validation-1-id1605 | 7.44 | 158.37 / 0.0213 | 241.18 / 0.0324 |
| en_us-validation-0-id1548 | 6.54 | 119.05 / 0.0182 | 175.28 / 0.0268 |
| en_us-validation-1-id1620 | 16.38 | 179.67 / 0.0110 | 253.36 / 0.0155 |
| pt_br-validation-0-id1565 | 11.82 | 161.19 / 0.0136 | 215.34 / 0.0182 |
| pt_br-validation-1-id1550 | 14.58 | 167.95 / 0.0115 | 253.38 / 0.0174 |
| cmn_hans_cn-validation-0-id1579-synthetic-noise10db | 18.38 | 168.12 / 0.0091 | 263.01 / 0.0143 |
| cmn_hans_cn-validation-1-id1605-synthetic-noise10db | 7.44 | 120.97 / 0.0163 | 232.68 / 0.0313 |
| en_us-validation-0-id1548-synthetic-noise10db | 6.54 | 62.58 / 0.0096 | 166.30 / 0.0254 |
| en_us-validation-1-id1620-synthetic-noise10db | 16.38 | 151.36 / 0.0092 | 223.61 / 0.0137 |
| pt_br-validation-0-id1565-synthetic-noise10db | 11.82 | 125.30 / 0.0106 | 202.90 / 0.0172 |
| pt_br-validation-1-id1550-synthetic-noise10db | 14.58 | 143.86 / 0.0099 | 231.92 / 0.0159 |
| zh-synthetic-long-public | 59.08 | 537.21 / 0.0091 | 1291.37 / 0.0219 |
| en-synthetic-long-public | 58.92 | 446.68 / 0.0076 | 929.01 / 0.0158 |
| pt-synthetic-long-public | 52.80 | 1565.50 / 0.0296 | 783.04 / 0.0148 |

## European-Portuguese audio, independent-ASR disagreement only

| MINDS pt-PT case | Seconds | Base edits/publisher units | Turbo edits/publisher units | Base median ms | Turbo median ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| minds-pt-PT-train-0 | 9.045375 | 17/20 | 0/20 | 105.76 | 360.83 |
| minds-pt-PT-train-1 | 4.693375 | 4/9 | 0/9 | 43.50 | 162.69 |

Turbo agreed with the publisher's ASR transcript on both clips; that does not establish zero human transcription error. Native-speaker correction/assessment of these recordings remains outstanding. English MINDS text also cannot serve as a human translation gold standard.

## Process memory and execution receipts

| Run | Model load ms | OS peak working set MiB | Final private commit MiB |
| --- | ---: | ---: | ---: |
| FLEURS Base | 685.42 | 298.25 | 1542.59 |
| FLEURS Turbo | 1658.85 | 428.11 | 2859.03 |
| MINDS pt-PT Base | 682.08 | 169.43 | 979.01 |
| MINDS pt-PT Turbo | 3860.40 | 330.42 | 2742.27 |

Windows reports process lifetime peak resident working set and snapshot private committed virtual memory. Neither is GPU memory. Fresh processes separate model peaks; disk/driver caches were not cleared. Model-only timings exclude recording, networking, VAD and insertion. All four native test processes passed,0failures. Test executable SHA256: `d87652e7be8b30a8a61828c8844b7837e84299c85f6f59d6181f879317877594`. Base and Turbo model hashes are recorded in the [synthetic stress report](2026-10-21-quality-stress.md).

Local receipts: `.context/quality-corpus/base-results.json`, `turbo-results.json`, `ptpt-base-results.json`, `ptpt-turbo-results.json`; manifests `manifest-benchmark.json` (15FLEURS cases) and `manifest-ptpt.json` (2MINDS cases). FLEURS manifest SHA256: `18fd0dc910bc6ea769845df8db20d94d9e99949135b0872936f983947b280edb`. Each result contains full measured runs, independent-reference edit counts, hypotheses and process-memory snapshots. No application source changed for these runs; no new installer was produced.

Portable acquisition/derivation is preserved in `scripts/fetch-quality-corpus.py`. On a fresh machine with Python3 and ffmpeg already available, run `python scripts/fetch-quality-corpus.py`. It uses the canonical public API, validates regional metadata, caps total audio transfer at10MB, converts WAVs and recreates the15FLEURS/2MINDS manifests. It refuses to overwrite existing corpus files and never persists the viewer's ephemeral signed asset URLs. For an existing cache, `python scripts/fetch-quality-corpus.py --reuse` checks hashes/formats without network access; syntax validation and this reuse check passed on all17current PCM16WAVs. The portable fetch path was not rerun against an already valid cache merely to test it.

To run either corpus through the existing fresh-process benchmark runner from PowerShell:

```powershell
$env:LECTUS_BENCH_MANIFEST = (Resolve-Path '.context/quality-corpus/manifest-benchmark.json').Path
cmd /c scripts\benchmark-model-quality.cmd
# For the separate pt-PT comparison, replace manifest-benchmark.json with manifest-ptpt.json.
Remove-Item Env:LECTUS_BENCH_MANIFEST
```

This runner writes `.context/quality-stress/<model>.json`; copy those receipts to a distinct run folder if preserving several runs. Read the manifest provenance before interpreting metrics: MINDS is still an automatic-reference comparison. API metadata is mutable; check downloaded source hashes against the table below. Different ffmpeg builds can change converted WAV header bytes, so the script records fresh source/input hashes rather than pretending every machine produces an identical container header.

## Exact clean corpus hashes

| Dataset case | Original WAV SHA256 | PCM16 input SHA256 |
| --- | --- | --- |
| cmn_hans_cn-validation-0-id1579 | `f3e12a114bedbc867e06bfb7aceb2721701aa729685b9e4be39f2b98305b72ab` | `6447c5fa59665e93dfca8aad4f3b4a65acc1542d333e0a292ca48b9e600c8a46` |
| cmn_hans_cn-validation-1-id1605 | `577a635c2261680c61dda5fecbc4f83bff6f37f453665b8d9d1a68f210f57c0a` | `cf5b35eff9ee8ce1dbe355a30d22f6edd0581822ac73aa5d10f5382a70c5407e` |
| en_us-validation-0-id1548 | `048169439c0328b758349af74aeb54a82a7a476e309709eb791dced06db25d48` | `77e7799b5b54ca6f4ac1f1ee3bf961f0beab4c797a8f74b911b262a5adf3b869` |
| en_us-validation-1-id1620 | `5fefdcd12d4c136762cc7841b084126fc60c6d6410f983cc9f82b6cfc45737a2` | `59b0da105997071cc3e8d51fc459b64a77e55b0e1cd052a3e96f9d538a256536` |
| pt_br-validation-0-id1565 | `36fe5f6b7c9e613d467797c7ffdd2b473898d784250bdb37fcacbb99dd68ceaf` | `d6e81039b26df55eab955f2519a9fbb43a598593ad7d80ff89734587cf5cea18` |
| pt_br-validation-1-id1550 | `200f56e8a83626644ed55a447d776313da575a1e00d2931113a8bcaadba55203` | `9295481c0304fd65f69ede7ac98b7c5fb4e3470fa7b9ae13d11f846c376d9ac9` |
| minds-pt-PT-train-0 | `fc084982ad50c6ea6cf066f08374b9b3aaa628d9a9accb167be5ae9376dbd275` | `56af35e3a40c3adca44690136f7dfcdb94edfd2e944d637965ad8f67d6559e00` |
| minds-pt-PT-train-1 | `aacee91914f902b0949425ee29ac984c48a34df55e5c195e65e0c8ff66977484` | `332b7bcffc4351acf85eb9b68152570887ae17384775ebb6749b0b99c0498581` |

Remaining limits: two recordings per regional configuration, no native-speaker adjudication, no speaker-balanced accent coverage, synthetic rather than natural noise/long conversations, and no alternate ASR family. Qwen3-ASR remains unintegrated. Translation is a separate text-generation task; these recognition measurements do not assess translation adequacy, and no model is declared best.
