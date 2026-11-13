# Download process restart diagnostic

Run `powershell -NoProfile -File scripts/diagnostics/download-resume-process/run.ps1` from a Visual Studio x64 developer environment. The standalone Cargo target is isolated in `.context/download-resume-process-target`; `-SkipBuild` reuses it.

The harness includes `src-tauri/src/model_download.rs` directly. A slow loopback HTTP fixture sends deterministic bytes. The launcher terminates only its first downloader PID after partial bytes and a synced receipt exist. A new downloader must request exactly that offset using Range/If-Range, finish the file and verify SHA256 before the harness publishes it. A GUID directory in `.context` retains request logs, child output and `result.json`; no recursive cleanup runs.

This tests the production transfer across actual process termination. It does not exercise closing Lectus, Tauri commands/events, native UI, power loss, the real app-data directory or the installed application. Digest checking and publication in this standalone harness mirror the native invariant but do not invoke Tauri's publication helper. No external network requests are needed after cached dependencies are available.
