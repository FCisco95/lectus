# Lectus

> Voice dictation for any app. Hold a key, speak, and your words land in whatever text field you were already in.

**Lectus** (by Organic) is a Whisper-powered dictation app for Windows and macOS.
Transcription runs **on your own machine** by default — your audio does not leave
it. The name nods to *lect-* (dialect, lecture — speech); the mascot is an
Eclectus parrot. Built with Tauri 2 (Rust) + React.

## You need to hold ORGANIC

Lectus is free for people who hold **ORGANIC (ORG)**. There is no account, no
subscription, no card, and no word limit.

| | |
|---|---|
| **Token** | ORGANIC (ORG) on Solana |
| **Mint** | `DuXugm4oTXrGDopgxgudyhboaf6uUg1GVbJ6jk6qbonk` |
| **You need** | **$20 worth**, held in a wallet you control |
| **Wallets** | Phantom, Solflare, Backpack |

You link a wallet **once**, by signing a message. That signature is free, it is
not a transaction, and it **cannot move SOL, tokens, or approve anything** — it
only proves the wallet is yours. Lectus then reads that wallet's ORGANIC balance
directly from the Solana chain.

The floor is in **dollars, not tokens**, so the number of ORG needed follows the
live price. Nothing is ever sent anywhere: the balance check is a public
read-only query, and the only thing stored is your wallet's public address.

**If you sell:** dictation keeps working for **7 more days**, with a banner. Top
back up inside that window and nothing is interrupted. Lectus never cuts off a
dictation in progress, and an internet outage cannot lock you out — the 7 days
are counted from the last time your wallet was actually *seen* above the floor,
so a check that never got an answer costs you nothing.

## Install

1. Download the newest installer from the
   [**Releases**](https://github.com/FCisco95/lectus/releases) page:
   - **Windows** — `Lectus_x.y.z_x64-setup.exe` (or the `.msi`)
   - **macOS (Apple silicon)** — `Lectus_aarch64.app.tar.gz`; unpack it and drag
     `Lectus.app` into `/Applications`
2. Run it. Lectus lives in the **system tray** (Windows) / **menu bar** (macOS)
   and starts with your computer, silently.
3. Follow the setup flow:
   - **Link your wallet** — opens your browser, where your wallet extension
     lives, and asks for one signature.
   - **Microphone access** — grant it when your OS asks.
   - **Accessibility** (macOS only) — required to type into other apps.
   - **Pick your dictation key** — the default is **hold Right Ctrl**.

On first run Lectus downloads the multilingual Whisper model (~140 MB) in the
background. Until it lands, a smaller English-only model that ships with the app
covers you.

Updates install themselves: Lectus checks on launch, downloads in the background,
and offers a **Restart now** banner. Nothing happens without your click.

## Using it

**Hold your key, talk, let go.** The text appears where your cursor already was —
any app, any field. That is the whole product.

- **Hold mode** (default) — push-to-talk. Hold Right Ctrl while you speak.
- **Toggle mode** — tap once to start, tap again to stop, for long dictations.
- Click the **floating pill** or the tray icon to open **Home**: your word counts
  for the week and every past dictation, searchable and copyable.

Worth turning on, under Settings:

- **Vocabulary** — names and jargon you want spelled right ("Mycel", not "my cell"),
  plus find/replace rules applied after every transcription.
- **AI cleanup** — punctuation, capitals, and filler removal. Runs through the
  Claude Code CLI on your own subscription, or locally with Gemma.
- **Language** — auto-detect by default; force one (e.g. English inside a code
  editor) globally or per app.
- **Mute while dictating** — silences background playback while your key is held.
- **Cloud backend** — optional Groq `whisper-large-v3-turbo` for more accuracy.
  Off by default; your audio stays local unless you turn it on and add a key.

## Membership questions

**Where do I see my status?** Settings → **Membership**. It shows the linked
wallet, what it holds, what $20 is in ORG today, and a **Check again** button.

**How often is it checked?** Every 12 hours in the background. Your wallet is
never asked to sign again.

**I hold ORG but it says locked.** Hit **Check again**. If the public Solana RPC
is rate-limiting you, point Lectus at your own node by setting the environment
variable `LECTUS_RPC_URL` (for example, a Helius URL).

**What is stored about me?** Your wallet's public address, its last known
balance, and when it was last checked — in `license.json` on your own machine.
Nothing is uploaded. **Unlink** in Settings deletes it.

**Is this DRM?** No. Lectus is open source and the binary can be patched. The
gate exists so that "Organic holders get Lectus" is actually true — not to fight
anyone. See
[the spec](docs/superpowers/specs/2026-09-17-organic-token-gate.md) for exactly
how it behaves, including every failure path.

---

## For developers

```bash
npm install
npm run tauri dev
```

The multilingual model downloads on first run; `scripts/download_model.sh`
(or `.ps1`) fetches it ahead of time.

**Tech:** Tauri 2 · Rust (cpal, whisper-rs, llama-cpp-2, ed25519-dalek, reqwest,
arboard, enigo) · React 18 + TypeScript · Vite.

### Build & deploy (Windows)

```cmd
rem 1. Release build into C:\lt\release (vcvars + Vulkan SDK env)
build-release.cmd

rem 2. Deploy that build over the installed app and relaunch it
deploy-local.cmd
```

`deploy-local.cmd` kills any running `chirp.exe` (UAC-elevated kill as fallback — an
elevated instance refuses a normal `taskkill`), copies exe + DLLs from `C:\lt\release`
into `%LocalAppData%\Lectus`, and relaunches. Desktop shortcuts always target the
install dir, so after this they run the fresh build.

### Releases

Tag-driven: push a `v*` tag → GitHub Actions builds Windows + macOS, signs, and
publishes a draft release with `latest.json` for the in-app updater.

```sh
node scripts/bump-version.mjs 0.5.1   # package.json + Cargo.toml + tauri.conf.json
git commit -am "chore: bump to 0.5.1"
git tag v0.5.1 && git push origin master v0.5.1
```

The workflow fails early if the tag and the three manifests disagree. Review the draft
release on GitHub, then publish it: installed apps pick it up on next launch (background
download, "Restart now" banner in Settings). Windows bundling of the llama/ggml DLLs +
`backends/` lives in `src-tauri/tauri.windows.conf.json`; macOS in `tauri.macos.conf.json`.
