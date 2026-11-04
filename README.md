<img src="docs/portfolio/media/lectus-logo.png" alt="Lectus Eclectus parrot logo" width="80">

# Lectus

**Speak into the text field you already use.**

Writing a message, note, or draft should not require switching to a transcription app and copying the result back. Lectus records from a global shortcut, transcribes your speech, and inserts the text into the focused field. Local Whisper transcription is the default; cloud processing is optional.

**Access before installation:** the distributed app requires a linked Solana wallet holding **at least $20 worth of either ORGANIC (ORG) or Mycel (MYCEL)**. The threshold follows the token price. Wallet linking signs an ownership message; it is not a token transfer. See the [membership and installation guide](https://github.com/FCisco95/lectus-releases#readme) before downloading.

[Download v0.7.1](https://github.com/FCisco95/lectus-releases/releases/tag/v0.7.1) · [User guide](https://github.com/FCisco95/lectus-releases#readme) · [v0.7.1 source](https://github.com/FCisco95/lectus/tree/v0.7.1)

## Try it

The public distribution checked on **4 November 2026** is **v0.7.1**, published on 11 October. Installers are hosted in **lectus-releases**; the release in this source repository contains only an updater feed. The default `master` branch and the release tag differ: use the tag when inspecting this version.

| Platform | v0.7.1 asset | Requirements |
|---|---|---|
| Windows x64 | `Lectus_0.7.1_x64-setup.exe` or `Lectus_0.7.1_x64_en-US.msi` | Microphone access; an editable target field |
| macOS, Apple silicon | `Lectus_aarch64.app.tar.gz` | Microphone and Accessibility permissions; an editable target field |

The distribution guide documents OS security prompts for apps without Microsoft/Apple publisher signing. Updater signature files are a separate mechanism. No Intel Mac or Linux installer is listed in this release.

1. Read the access requirements above, download the matching asset, and follow the user guide. Keep the app in the system tray or menu bar.
2. Complete wallet linking and OS permissions. Choose your dictation shortcut in Settings and wait for the selected model to be ready. Model downloads and membership verification need network access.
3. Focus a blank note, hold your configured shortcut, say “The project review is on Thursday,” and release. In toggle mode, press once to start and again to stop. Check the inserted text before using it.

Insertion depends on the target app, focus, and OS permissions. Recognition varies with microphone, language, and model; names, numbers, and negation need review.

## What v0.7.1 includes

- **Dictation in your existing apps:** hold or toggle recording, a floating status pill, and local Whisper models.
- **Vocabulary and language controls:** recognition hints for names and jargon, exact replacement rules, language selection, and per-app overrides.
- **Local history:** browse, copy, and filter previous transcripts by text; this is substring search.
- **Optional processing:** Groq cloud transcription and optional AI cleanup. Local cleanup needs its model; remote cleanup needs its provider setup. Cleanup can change wording, so review the result.
- **Paired-computer text delivery:** v0.7.1 adds encrypted LAN delivery of the finished transcript, with local dictation remaining the default. Both computers need this version and permissions. [The versioned guide](https://github.com/FCisco95/lectus/blob/v0.7.1/docs/REMOTE-DICTATION.md) records the remaining Windows-to-Mac insertion and installation checks; published packages do not establish end-to-end acceptance on every setup.

New ChatGPT/Claude connection cards, explicit translation controls, and a local HTTP API under development are outside this release description. Development screenshots and mocked UI checks are not demonstrations of the downloadable version.

## Privacy depends on the backend

Local transcription does not make every configuration offline.

| Configuration | What is processed where |
|---|---|
| Local Whisper, cleanup off | Speech recognition runs on your machine. The pipeline stores transcript text in local history. |
| Local Whisper with local cleanup | Recognition and cleanup run on your machine, after the required models are downloaded. |
| Groq cloud transcription | Recorded audio is sent to the configured transcription endpoint. An API key and network access are required; provider usage may incur charges. |
| Remote cleanup | Transcript text is sent through the configured cloud chat endpoint or Claude Code CLI. Even with local recognition, this text leaves the local inference path; provider terms and usage limits apply. |
| Paired-computer mode | The finished transcript is delivered to the paired computer over the local network. Audio processing still follows the sender's selected transcription backend. Keep the connection code private. |

Membership verification queries external services for wallet balances and token prices; model downloads and update checks also use the network. History is stored as `history.json` in the app's data directory. The membership guide describes the seven-day grace policy and unlinking. This is a description of data paths in the release source, not a claim about a provider's retention policy or an independent security audit.

## Engineering

Lectus uses **Tauri 2, Rust, React, TypeScript, and Vite**. Rust owns audio capture, model inference, global input handling, text insertion, persistence, and paired delivery. React provides the settings and status UI through Tauri commands and events.

```text
Configured shortcut → microphone capture → local Whisper / cloud transcription
                    → vocabulary + replacement rules → optional cleanup
                    → local history → focused text field / paired computer
```

Local speech recognition uses `whisper-rs`; local cleanup uses `llama-cpp-2`. The release configures Vulkan on Windows and Metal on macOS. Native inference avoids uploading audio in local mode, at the cost of model downloads, memory use, and hardware-dependent latency. Clipboard/keyboard insertion integrates with existing apps but depends on their input permissions and focus.

The [v0.7.1 remote-dictation guide](https://github.com/FCisco95/lectus/blob/v0.7.1/docs/REMOTE-DICTATION.md#release-and-validation) records a frontend build, 156 native tests with five environment-dependent tests excluded, and a LAN discovery/delivery check. Those are historical, scoped results, not a fresh test run or proof of every installation path.

## Development

Inspect [the v0.7.1 source](https://github.com/FCisco95/lectus/tree/v0.7.1) to reproduce the version described here. The native build needs Node/npm, Rust, and the platform's Tauri tooling. The release workflow uses Node 22 and stable Rust; Windows also configures MSVC, Vulkan SDK, and Ninja. A frontend preview alone does not exercise microphone capture or native text insertion.

In a separate development checkout with those prerequisites installed:

```sh
npm ci
```

Provision the bundled English model using `scripts/download_model.ps1` on Windows or `bash scripts/download_model.sh` on macOS. This downloads a model; inspect the script before running it. Then:

```sh
npm run tauri dev
```

These commands are documented from the versioned source and were not executed for this documentation update. Native dependency/resource staging may require the platform steps in the [versioned build workflow](https://github.com/FCisco95/lectus/blob/v0.7.1/.github/workflows/release.yml). Running the app can use the normal Lectus configuration and request OS permissions; use a controlled development environment. `build-release.cmd` and `deploy-local.cmd` are machine-specific operational helpers that can replace and restart an installed app; they are not a general setup path.

For contributions, distinguish the branch, release tag, and platform you tested. Never include API keys, wallet connection codes, or personal dictation history in an issue or pull request.

## Distribution and licensing

Use [lectus-releases](https://github.com/FCisco95/lectus-releases) for installation guidance and [published release notes](https://github.com/FCisco95/lectus-releases/releases) for distribution changes. This repository contains the application source. No top-level license file was found at the reviewed `v0.7.1` source tag; do not infer a permissive license from public visibility.
