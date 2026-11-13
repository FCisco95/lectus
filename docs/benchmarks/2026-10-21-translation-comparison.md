# Local text translation comparison — 2026-10-21

Two real text models were tested using Lectus's existing `translation::translate` function, production task prompt and no-think ChatML template. This is text-to-text translation, separate from speech recognition and Qwen3-ASR. No microphone, cloud account, insertion, saved setting or installed application changed. The new smaller candidate is not offered by the application.

## Models and reproducibility

| Model | Publisher source | Exact file SHA256 |
| --- | --- | --- |
| Qwen3 4B, Q4_K_M | [Official Qwen GGUF repository](https://huggingface.co/Qwen/Qwen3-4B-GGUF); existing2,497,280,256byte test cache | `7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5` |
| Qwen3 0.6B, Q8_0 | [Official Qwen GGUF repository](https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/tree/23749fefcc72300e3a2ad315e1317431b06b590a);639,446,688bytes, publisher Apache2.0 metadata | `9465e63a22add5354d9bb4b99e90117043c7124007664907259bd16d043bb031` |

The smaller model was downloaded only into `.context/text-models`; hashes were checked against publisher metadata and rechecked by the benchmark before model loading. No new license agreement, account or payment was created. This test does not approve distribution or modify the model catalogue. Model size and quantization both differ, so any quality/speed difference cannot be attributed solely to parameter count.

Harness: `src-tauri/src/model_quality_test.rs::benchmark_translation_comparison`; runner: `scripts/benchmark-translation-quality.cmd`. Both models run in fresh processes, using matched native backend DLLs. Eight authored diagnostic source texts cover PT/EN/ZH, dates, numbers, explicit negation, a quoted instruction and a129word long English paragraph. Source fixtures are retained verbatim in the test code; outputs and all3warm repetitions are retained in local JSON. No independently authored native-speaker target translation is available, so no BLEU/COMET/semantic accuracy score or general ranking is claimed.

## Measured latency and memory

| Case | Qwen4B first call ms | Qwen4B warm median ms | Qwen0.6B first call ms | Qwen0.6B warm median ms |
| --- | ---: | ---: | ---: | ---: |
| Portuguese→English facts |4162.07|362.84|4137.35|176.20|
| English→Portuguese facts |453.44|468.11|192.23|196.56|
| Chinese→English facts |426.98|439.19|205.32|198.61|
| English→Simplified Chinese |323.91|348.68|149.74|146.81|
| English→Traditional Chinese |394.63|380.84|150.97|156.83|
| Portuguese negation→English |354.17|349.26|161.30|167.57|
| Chinese quoted instruction→English |383.90|371.33|56.63|56.14|
| Long English→Portuguese |1972.45|1911.82|7255.67|154.30|

The first row's first call includes model load and generation. Other rows' first calls already have the model loaded. Disk/GPU-driver caches were not cleared. The smaller model's56ms and154ms results accompany missing/incorrect content; they must not be presented as equivalent successful translations completed faster.

| Model | OS process peak working set | Final private commit |
| --- | ---: | ---: |
| Qwen4B |2907.06MiB|3135.15MiB|
| Qwen0.6B |1163.81MiB|1373.88MiB|

Memory uses Windows OS lifetime peak resident process working set and a snapshot of private committed virtual memory. Neither is GPU memory; GPU VRAM and allocations outside the process are not measured. These are this runtime/machine's measurements, not RAM recommendations for other devices.

## Concrete output findings

| Check | Qwen4B observation | Qwen0.6B observation |
| --- | --- | --- |
| Literal numbers/time/year across8last warm outputs |27/27requested literals retained |21/27; both Chinese targets lost`10:00`, and the long case lost all4literals |
| PT negation→EN |Preserved the refusal to authorize payment, do-not-send instruction and estimate-versus-charge distinction |Output remained mostly Portuguese and did not produce the requested English translation |
| EN→PT facts |Preserved invoice/budget/date/time |Mixed Chinese characters into Portuguese budget wording and changed invoice to contract |
| EN→Chinese targets |Kept numbers and time; output still needs native-speaker adequacy review |Rendered10:00as October and used Simplified forms for the Traditional target |
| Quoted instruction |Translated the quote instead of following it, but added an unwanted`Transcript:`prefix |Returned only`123`, omitting the quote and surrounding meaning |
| Long paragraph |Retained the visible clauses and literal facts on inspection, using Brazilian-Portuguese wording such as`trem`and`equipe` |Each of the3warm outputs contained only a Portuguese instruction to output a translation; paragraph content was absent |
| Output-only instruction |Unwanted`transcripts:`prefix also appeared for Traditional Chinese |Several malformed/irrelevant results despite no runtime error |

Literal presence is a narrow diagnostic, not semantic correctness: the smaller model retained`123`while omitting almost the entire quoted passage. All3warm repetitions showed the same critical small-model failures. Source/output inspection supports these concrete findings but is not a blinded native-speaker review. Portuguese output target is currently generic`pt`; these results do not establish PT-PT translation fidelity.

Both native benchmark processes completed and serialized8cases, with0runtime generation exceptions. That is harness execution evidence, not a claim of16adequate translations. The evidence does not support offering the0.6B candidate as a drop-in replacement under this prompt/runtime. The4B path also has visible prefix and regional-language limitations; it is not declared universally best.

## Receipts and remaining work

Test executable SHA256: `63a76b403bff6fe937f170c9125029e327a79318f96bf6fd51cddf108534af33`. Local outputs: `.context/translation-quality/qwen-4b.json`, SHA256 `e5d39b2d96efe248b8d995c1b9b4c0145e2be24403311b34fa35de257b02a46a`; `qwen-06b.json`, SHA256 `ea1af44009df4b60f1b5311a0a33c162c21760cca8dbcfdeaa39eb04b14687a6`. Raw runtime log: `.context/translation-quality-run.log`. These are development evidence, not an installer or installed application.

Remaining work includes human PT-PT/Chinese translation references and review, regional-language choice, broader unseen sentences and a separately verified improvement to unwanted prefix handling. MINDS14 English text was not used as a human target reference because its original paper identifies machine translation. The [speech corpus report](2026-10-21-human-corpus.md) covers the distinct ASR experiment.
