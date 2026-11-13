# Model download recovery — 2026-10-21

State: local development changes only, uncommitted; no installation, installer, model in real app data, public release or capture/insertion/shortcut change.

## Reproduced failure

The old `download_with_progress` opened its existing `.part` with `std::fs::File::create`. Before changing it, a standalone Rust reproduction of that exact statement wrote a 200,000-byte partial, opened it for retry, and wrote five bytes: `before=200000, after=5`. This demonstrates truncation of saved progress; it was not a native app UI acceptance test.

## Implemented

- Shared resumable transport for Whisper, VAD, optional Gemma cleanup and checksum-bound Qwen translation. A `.part.json` receipt binds the original URL, publisher checksum, total size and strong ETag (or Last-Modified). Existing receipt-less legacy partials cannot be safely resumed and restart on retry with an explicit message.
- A retry sends Range and If-Range, validates HTTP 206 Content-Range/start/end/total/body length and validator before append. A full HTTP 200 safely restarts instead of mixing bytes. Invalid ranges preserve existing bytes but invalidate remote metadata so a subsequent retry can recover. Missing size, compressed content, small/error responses and bad server statuses fail safely.
- Cancellation interrupts both a pending request and a stalled body read through async cancellation polling (100 ms interval). Partial bytes and receipts survive cancellation, connectivity errors, write errors and app restarts. Existing download commands resume; a new cancel command is integrated by the coordinator. Typed voluntary pause is distinct from an error.
- Free capacity is checked against remaining download bytes plus 1 MiB using the operating system's available-to-user measurement. Write/sync errors remain failures with a recovery instruction. This check is advisory because other programs can consume space during a transfer.
- Every supported Whisper/VAD/Gemma download now requires a pinned publisher SHA256, extending the existing Qwen integrity guarantee. A complete download remains `.part` until the digest passes, the cancellation/publication decision is serialized and the file is atomically renamed. Progress remains at most 99% until publication. Previously present legacy models also require verification, cached by file metadata. Invalid bytes are quarantined, never advertised as ready.
- DownloadStatus adds paused, downloaded_bytes, total_bytes and message while retaining downloading/progress/error. `download_status_for(app,name)` reconstructs saved progress on restart. Failure status retains errors, including when resumable progress exists.
- Removed unsupported precision/speed claims from the Tiny, Base and Medium catalog descriptions.

## Verification evidence

1. Native focused release suite: 16 passing tests, zero failures (seven existing integrity/locking tests and nine new transport tests). Local HTTP fixture servers exercise actual TCP/HTTP plus filesystem writes: resumed prefix preservation, explicit Range/If-Range, ignored Range or changed remote object, invalid/missing ranges and changed validators, interrupted body then successful resume, cancel after bytes, cancel during a stalled response, Last-Modified fallback and incompatible legacy/checksum receipts. These are controlled HTTP fixtures, not proof of the native UI or every production CDN.
2. Opt-in native **real publisher download**: `cargo test --release --lib live_vad_download_cancel_resume_and_verify -- --ignored --nocapture` passed. It used only a temporary directory, paused the official 885,098-byte Silero VAD download at 8,192 bytes, resumed exactly at 8,192, verified publisher SHA256 `29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf`, then published only into the disposable directory. Runtime 1.47 seconds. Test remains ignored by default because it requires external connectivity.
3. A separate live HTTP probe returned 206 for bytes 0–15/885098, and then 206 for bytes 16–31/885098 with the same ETag and If-Range. This confirms current resume support on the official VAD route, not all remote hosts forever.
4. Insufficient-space budget tested with synthetic capacity. The live test exercised the real Windows free-space query. **No real disk was filled** and a constrained filesystem quota/write-time ENOSPC was not independently reproduced in this round. Native UI cancellation, app termination during transfer and process/power-loss recovery still need controlled acceptance.

Focused tests compiled using the existing Visual Studio x64 / Vulkan / release toolchain, without changing global tools or staging/installing the app. Existing unrelated warnings persisted. The coordinator owns integrated full-suite/build evidence after all agents' changes.

## Official pinned publisher digests

Retrieved 2026-10-21 from each publisher repository's `?blobs=true` API; LFS SHA256 identifies exact bytes even though the download URLs use mutable main. A changed upstream file is intentionally refused until the catalog is deliberately reviewed/updated.

Sources: [Whisper model publisher API](https://huggingface.co/api/models/ggerganov/whisper.cpp?blobs=true), [VAD publisher API](https://huggingface.co/api/models/ggml-org/whisper-vad?blobs=true), [Gemma GGUF publisher API](https://huggingface.co/api/models/unsloth/gemma-3-1b-it-GGUF?blobs=true).

| File | Exact size | Publisher SHA256 |
|---|---:|---|
| ggml-tiny.bin | 77,691,713 | be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21 |
| ggml-base.bin | 147,951,465 | 60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe |
| ggml-small.bin | 487,601,967 | 1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b |
| ggml-medium.bin | 1,533,763,059 | 6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208 |
| ggml-large-v3-turbo.bin | 1,624,555,275 | 1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69 |
| ggml-large-v3-turbo-q5_0.bin | 574,041,195 | 394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2 |
| ggml-large-v3.bin | 3,095,033,483 | 64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2 |
| ggml-silero-v5.1.2.bin | 885,098 | 29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf |
| gemma-3-1b-it-Q4_K_M.gguf | 806,058,272 | 8270790f3ab69fdfe860b7b64008d9a19986d8df7e407bb018184caa08798ebd |

Gemma agreement enforcement remains intact. No real model agreement was accepted by these tests. The real model test uses VAD only.

## Integration review follow-up

The translation-model catalog now reuses `translation::LOCAL_MODEL_NAME` / `LOCAL_MODEL_SHA256` directly so its verified file remains visible to the shared availability check (ten supported models including the earlier pinned Qwen). A regression covers that integration. `can_pause` is explicit: true as soon as a transport token exists, including connection establishment; false during legacy-file-only verification and after errors/completion. Status reconstruction preserves an existing recovery message/error. Resume of a compatible partial preserves its already validated receipt without rewriting it; initial receipt bytes are synced before the response body is stored. These follow-up edits await the coordinator's integrated native test run; the 16-test focused result above preceded them.

A read-only cross-review of the isolated installer/recovery work found two follow-ups communicated to its owner: missing exclusion of concurrent installer transactions and raw relative-path alias handling (`sub/../backups/...` can normalize into a reserved namespace). No installer file was changed by the download worker; final resolution is recorded by the coordinator.

Known Gemma/Qwen legacy paths are now verified inside `local_llm::load_into_slot`, after the separate Gemma agreement gate and before either resident-cache reuse or file loading. The helper resolves catalogued basenames case-insensitively, holds the existing per-path lock and reuses digest/metadata verification. A new regression rejects and quarantines two 200 KB legacy fixtures (including uppercase Qwen filename), avoiding large model fixtures. Explicit unknown experimental benchmark filenames retain their earlier loading behavior and do not receive catalogued integrity claims. Lock order was reviewed: local LLM resident slot may acquire a download path lock; downloads/verifiers do not acquire the resident slot, and ensure/download returns before callers warm the model. This follow-up awaits integrated verification.

## Concurrent download review fix

Independent review reproduced a queued-retry race: a second ensure/download blocked on the file lock and restarted a fresh transfer after the first was paused. Download admission now uses a nonblocking reservation and rejects concurrent download/verification requests immediately, before changing status or creating another token. An explicit request after the pause can still reserve the lock and resume. A two-thread regression holds the first permit, requires the duplicate to reject before that permit is released, checks that releasing the paused operation starts nothing else, and permits a later explicit resume. This change awaits a focused/integrated test rerun after the already-running previous-source build completes.

## Actual helper-process termination and restart — 19:14 UTC

Coordinator integration evidence: the final full native suite after the concurrent-admission, Qwen catalog, legacy-file and explicit pause-capability fixes passed **244 tests, 0 failures, 10 ignored, 2 OS-mutation exclusions**, including the later test-only translation comparison harness. The development release build also passed. Final frontend behavior includes a RED→GREEN fixture proving voluntary pause does not become a failure alert when the original IPC request returns cancellation. The concurrency/alias/lifecycle findings in the installer cross-review were corrected and its owner verified 13 fixtures on both Windows PowerShell 5 and PowerShell 7. Exact artifacts and source receipts are tracked in the aggregate reliability review; no app installation is implied.

A separate diagnostic now exists at `scripts/diagnostics/download-resume-process/` (Cargo manifest/lock, Rust launcher/server, PowerShell runner and scope README). It directly includes the **unchanged production** `src-tauri/src/model_download.rs`, builds offline into a separate `.context/download-resume-process-target` and never uses the app executable, app data or installed resources.

`powershell -NoProfile -File scripts/diagnostics/download-resume-process/run.ps1 -SkipBuild` passed at `2026-10-21T19:14:38.5854287Z` after its independent build. A slow real loopback HTTP server supplied deterministic 2,000,000-byte test data. The runner terminated only downloader child PID 49900 without calling its cancellation token, after partial bytes and receipt existed. A new downloader process resumed exactly at the remaining partial's 139,264-byte offset. Captured requests confirmed offset 0 for the first process, then `Range: bytes=139264-` and `If-Range: "process-fixture-v1"` for the new process. There was no published destination after termination. The second process verified final SHA256 before the harness renamed the completed partial; the runner independently rechecked the full file and absence of partial/receipt.

Portable decisive evidence:

```json
{
  "Passed": true,
  "TimestampUtc": "2026-10-21T19:14:38.5854287Z",
  "ProductionTransportSHA256": "32C50D860327B1F933F174C5F8E5C7B0693336EA797EF5567F9BDBF5CA13FBCA",
  "SavedBytes": 139264,
  "ResumedFrom": 139264,
  "FinalBytes": 2000000,
  "SHA256": "82fa05417c03925cb7e8fd2bc2e9f2e2a1c8c421427ccdba1ab0091261e3a840"
}
```

Local raw evidence remains at `.context/download-resume-process-eec9449f14a3441faf39adbed0e37ef2/` (`requests.jsonl`, `result.json`, child output and deterministic fixture). The standalone executable is a **diagnostic**, not an app candidate or installer.

Scope: this proves the actual production transport can recover saved progress across child termination and a fresh process. It **does not prove closing/restarting Lectus**, Tauri state/events, native UI, hardware failure or power-loss recovery. The standalone harness performs its own checksum-before-rename publication; native app publication remains covered by the integrated tests separately. The initial controller run encountered Windows PowerShell's missing ExitCode after a short-lived redirected child exited; preserving the process handle corrected the controller, without any runtime/source change. The successful rerun above independently verifies every outcome.
