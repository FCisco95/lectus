# Lectus

> Voice dictation for any app. Hold a key, speak, and your words land in whatever text field you were already in.

**Lectus** (by Organic) is a Whisper-powered dictation app for Windows and macOS.
Transcription runs **on your own machine** by default — your audio does not leave
it. The name nods to *lect-* (dialect, lecture — speech); the mascot is an
Eclectus parrot. Built with Tauri 2 (Rust) + React.

**Download, install, membership and usage:** see the public
[**lectus-releases**](https://github.com/FCisco95/lectus-releases) repo — that README is
what users read (the in-app Help button opens it). This repo is the source.

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
publishes a draft release with `latest.json` for the in-app updater — in the
**public** [`lectus-releases`](https://github.com/FCisco95/lectus-releases) repo, so this
one can stay private. CI writes there with the `RELEASES_TOKEN` secret (fine-grained PAT,
Contents: write on `lectus-releases` only; it expires — renew it before tagging).

```sh
node scripts/bump-version.mjs 0.5.1   # package.json + Cargo.toml + tauri.conf.json
git commit -am "chore: bump to 0.5.1"
git tag v0.5.1 && git push origin master v0.5.1
```

The workflow fails early if the tag and the three manifests disagree. Review the draft
release on GitHub, then publish it: installed apps pick it up on next launch (background
download, "Restart now" banner in Settings). Windows bundling of the llama/ggml DLLs +
`backends/` lives in `src-tauri/tauri.windows.conf.json`; macOS in `tauri.macos.conf.json`.

**One-time bridge (the first release after the move only).** Installs of 0.6.0 and older
look for updates in *this* repo. After publishing the first `lectus-releases` version, copy
its `latest.json` here so they can find it:

```sh
gh release download v0.7.0 --repo FCisco95/lectus-releases --pattern latest.json
gh release create v0.7.0 latest.json --repo FCisco95/lectus --title "Lectus v0.7.0" \
  --notes "Releases moved to https://github.com/FCisco95/lectus-releases"
```

The download URLs inside that `latest.json` already point at `lectus-releases`.
Keep this repo public until existing installations have updated to 0.7.0; verify the
version on your Windows and Mac installs before changing visibility. Installs still on
0.6.0 after this repo becomes private will need to update manually from `lectus-releases`.
