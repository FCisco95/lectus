# Offline translation smoke benchmark — RTX 3080

Model: official `Qwen/Qwen3-4B-GGUF`, `Qwen3-4B-Q4_K_M.gguf`, 2,497,280,256 bytes. SHA256 `7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5`, verified against the publisher's Hugging Face file metadata. Test cache is `models/Qwen3-4B-Q4_K_M.gguf` (git-ignored); it was not installed into the user's app data or selected in their running app.

The existing llama.cpp runtime ran real local generation with no cloud request. Fixture meaning: a meeting on Thursday at 10:00, with a budget of 250 euros. Assertions check preserved numbers and basic language output; displayed translations were also inspected. This is a small smoke test, not a human/native-speaker quality evaluation or a ranking against alternative models.

| Direction | Warm second-pass time | Observed output |
|---|---:|---|
| Portuguese → English | 235 ms | The meeting is on Thursday at 10:00. The budget is 250 euros. |
| Chinese → English | 241 ms | The meeting is held on Thursday at 10:00. The budget is 250 euros. |
| English → Simplified Chinese | 206 ms | 会议在周四上午10:00举行。预算为250欧元。 |
| English → Traditional Chinese | 250 ms | 會議於星期四上午10:00舉行。預算為250歐元。 |
| English → Portuguese | 286 ms | A reunião é quinta-feira às 10:00. O orçamento é 250 euros. |

Initial cold execution took 11,481ms for the first model load/generation and 10,041ms for one new context shape; after GPU shader caching, a fresh-process first translation took 2,072ms. Do not promise warm timings on first use or other hardware. The final two-pass run passed in 4.23s.

## Runtime defect discovered and fixed

The first run failed because a moved build still referenced `C:/lt` and the test executable lacked its runtime backend folder. Staging the matched backend DLLs allowed loading, but llama still saw an empty backend registry. Whisper statically links a second ggml; the generic Rust loader symbol registered backends into that registry, not the shared ggml used by llama.dll. Windows now resolves `ggml_backend_load_all_from_path` explicitly from the loaded `ggml.dll` with the signature confirmed against the vendored header. The subsequent actual model test passed.

Development tests need the matching native backends beside the executable (`target/release/deps/backends`); the packaged app uses its `backends` resource directory. Staging used only the native build whose llama.dll hash matched the release runtime. No global loader paths or installed binaries were changed.

Reproduce after staging native backends: `scripts/verify-native.cmd --translation-bench`. The raw log is `docs/benchmarks/2026-10-21-translation.log` locally (ignored); this file preserves the key evidence portably.
