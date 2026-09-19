# Lectus - HANDOFF

## Metadata

- Last Updated: 2026-09-17 afternoon (Windows PC)
- Repository: `lectus` (github.com/FCisco95/lectus) — **public** since this morning
- Branch: `master` at `d603a89`, pushed, clean
- Version in manifests: `0.6.0`, **released and published** (updater feed live)
- Live Windows install: `%LocalAppData%\Lectus\chirp.exe` (gate build, 2026-09-17 12:05)

## TL;DR

The **Organic token gate** is built, tested live, and shipped. Lectus is free for
people holding **$20 of ORGANIC**; the wallet is linked once by signing a nonce,
and the balance is re-read every 12 h. No account, no subscription.

## Mac development setup (2026-09-19)

Fresh development clone is ready on macOS: `npm ci` completed, `npm run build`
passes, and `scripts/download_model.sh` downloaded the ignored 74 MB
`models/ggml-tiny.en.bin` fallback model. Node 24.14.0, npm 11.19.1, Rust 1.94.0,
and Xcode Command Line Tools are installed.

The existing checkout is registered in Orca as the `lectus` project, linked to
`github:fcisco95/lectus`. Its main `master` worktree is visible and marked
in-progress; no duplicate checkout or child worktree was created.

`cargo test --release --lib` compiles successfully but is **not fully green on
macOS**: 111 tests pass, 4 are intentionally ignored, and two Windows-autostart
tests fail. `src-tauri/src/autostart.rs` uses Windows-style backslash paths;
on macOS `PathBuf::file_name()` treats each test fixture as one filename, so the
expected Windows repair branch is not reached. This is a test-portability issue,
not a setup failure. No source changes were made. Production npm audit reports
zero vulnerabilities.

Earlier the same morning: the Home window landed (tray / shortcut / pill open it,
login silent via `--autostart`), and the repo was made public.

Specs: `docs/superpowers/specs/2026-09-17-organic-token-gate.md`,
`docs/superpowers/specs/2026-09-17-home-window-design.md`.

## Shipped since (same day, afternoon)

- **v0.6.0 released and published.** CI run `35215811370` green on all three jobs.
  `latest.json` returns 200; the in-app updater works. The stale v0.5.0 draft was
  deleted (tag kept). The locally-installed exe still reports 0.5.0 — it was copied
  in before the version bump — so **this machine can test the updater end to end**,
  which has never been proven.
- **README rewritten for holders, not developers** (commit `722d993`).
- **Vocabulary + rules loaded** into `config.json` (64 terms, 30 rules). Two code
  constraints drove the shape, and both still apply to any future additions:
  `apply_dictionary_spellings` force-cases whole words, so a lowercase entry would
  destroy sentence-start capitals — dictionary holds **proper nouns and ALL-CAPS
  acronyms only**. And Whisper's `initial_prompt` caps near 224 tokens, so the list
  cannot grow without bound. Backup at `config.json.bak`.
  Real mishearings found in the user's own history: `fecheiros`→ficheiros (26×),
  `Hifi`→Hyphae, `cloud code`→Claude Code.

## Decisions made (do not re-litigate)

1. **No subscription, no email accounts.** Transcription is local, so there is no
   metered cost to bill for; a subscription would be fake SaaS. The wallet *is* the
   account. If settings portability is ever wanted: export/import file first,
   wallet-keyed sync second, email accounts never (unless cloud ASR arrives).
2. **Not selling this as a product for now.** Holding ORGANIC stays the gate. Build
   quality instead.
3. **Code signing is parked.** Azure Artifact Signing (formerly Trusted Signing)
   rejects individual developers outside the US/Canada, and the user is an individual
   in Portugal. Options if revisited: Certum individual OV cloud cert (~€150/yr,
   commercial use allowed), registering an ENI to unlock Azure at $10/mo and EV
   later, or Apple Developer at $99/yr which **is** open to individuals worldwide and
   fully fixes macOS. Certum's *Open Source* cert and SignPath Foundation were both
   ruled out: they forbid commercially-distributed software, and the token gate
   makes Lectus commercial.

## What to do next

**The user's chosen direction: expose Lectus as a local API so other apps — Organic
first — can use its speech-to-text.** Lectus already injects into any focused field,
so a web app can *receive* keystrokes; what it cannot do is *ask* for a transcript,
show live partials, or transcribe a file.

Agreed first slice (spec it before coding — this is a microphone with an HTTP door):

1. `POST /v1/transcribe` — audio in, text out. No streaming, no mic control yet.
2. Per-install token, shown in Settings, required on every call.
3. Explicit origin allowlist (Organic's domain), **never** a wildcard.
4. Off by default, visible indicator when enabled and when transcribing.
5. **No remote-triggered recording in v1.** The API accepts audio the caller sends;
   it does not open the user's microphone on command. Browsers allow cross-origin
   requests to localhost, so an unauthenticated endpoint would be a microphone any
   webpage could reach.

The loopback-server pattern from `src-tauri/src/license/connect.rs` is the model to
reuse — it already binds 127.0.0.1 on a random port and serves a one-shot exchange.

Still unstarted from earlier asks: Mycel dictionary aliases, installer wizard polish,
the three one-line gate decisions (skippable onboarding wallet step, no trial window
for `Unlinked`, untested public-RPC rate limits).

## The gate, in one screen

| | |
|---|---|
| Mint | `DuXugm4oTXrGDopgxgudyhboaf6uUg1GVbJ6jk6qbonk` (ORG, 6 decimals) |
| Floor | **$20**, in dollars — token count follows the live price (~7,930 ORG) |
| Grace | 7 days, counted from the last time the wallet was *seen above* the floor |
| Re-check | every 12 h in background, plus "Check again" in Settings → Membership |
| Balance | `getTokenAccountsByOwner`, public mainnet RPC, `LECTUS_RPC_URL` overrides |
| Price | Jupiter `lite-api.jup.ag/price/v3` |
| Storage | `license.json` in `%APPDATA%\ai.organic.lectus\` (never in `config.json`) |

Code: `src-tauri/src/license/` (`mod` logic+persistence, `verify` ed25519,
`chain` RPC+price, `connect` loopback server + page, `commands` Tauri surface).
Gate point: `gate_blocks_dictation` in `lib.rs`, called at both pipeline entries
and ahead of the hold chime/ducker.

**Everything fails open inside grace.** An RPC error is never read as "balance 0";
a zero price is rejected; a failed check leaves `last_ok_ms` untouched. The gate
can only decline to *start* a dictation, never interrupt one.

## Verified live (2026-09-17, from the app's own log)

| Time | Event |
|---|---|
| 11:13:18 | link page served on `127.0.0.1:60534` |
| 11:13:40 | signature verified → **Active**, 639,557 ORG ≈ $1,613 |
| 11:14:08 | dictation ran: 1.6 s audio → 295 ms transcribe → 8 ms inject |
| 11:14:19 | after Unlink → **refused: Unlinked** |
| 11:14:47 | empty wallet linked → **Locked, $0** (no grace — never funded) |
| 11:15:13 | holder wallet re-linked → **Active**, dictation works again |

Only **Grace** is unit-test-only; it needs a wallet to fall below $20 after being
above it. 115 Rust tests pass (`cargo test --release --lib` via the Vulkan recipe —
a plain `cargo check` fails in the debug cmake path).

## PC inventory (2026-09-17)

| Copy | Path | Role |
|---|---|---|
| Installed, gate build | `%LocalAppData%\Lectus\chirp.exe` | **live** (12:05) |
| Cargo target | `C:\lt\release\chirp.exe` | same build |
| Config/history/license | `%APPDATA%\ai.organic.lectus\` | `config.json`, `history.json`, `license.json` |

Autostart: `HKCU\...\Run\Lectus` = `...\Lectus\chirp.exe --autostart`.
Only `chirp.exe` + `chirp_lib.dll` change between builds; the ggml/llama DLLs and
`backends/` have been byte-identical since July.

## Constraints

- Do **not** change capture/injection/shortcut unless asked. Default hotkey is
  still hold Right Ctrl; pill click opens Home.
- API keys stay in env/config only. **No RPC key ships in the binary** — the repo
  is public and any embedded key would leak with every download.
- Prefer small, focused diffs.

## Suggested skills

- `handoff-memory` — this file
- `verify` — driving the live app; do not inject into user windows
- `babysit` — watching the v0.6.0 CI run
- `superpowers:brainstorming` — Mycel dictionary / installer wizard, still unstarted

## Generated artifacts this session

| What | Where it lives | Notes |
|---|---|---|
| Local fallback Whisper model | `models/ggml-tiny.en.bin` | 74 MB, gitignored by design; download again on a fresh machine with `scripts/download_model.sh`. |

## Next-session prompt

```
Lectus is cloned and set up locally: frontend build passes; the fallback Whisper model is present. The next product slice remains the local transcription API; do not touch capture, injection, or shortcut handling. The two failing autostart unit tests are macOS path-portability issues only.

Files: docs/HANDOFF.md, docs/superpowers/specs/2026-09-17-organic-token-gate.md, src-tauri/src/autostart.rs
Model: Codex Sonnet 5 — focused Rust/Tauri product planning and implementation.
Skills: handoff-memory

Spec the local `POST /v1/transcribe` API before coding: per-install token, explicit Organic origin allowlist, off by default with visible activity state, and no remote microphone control in v1.
```
