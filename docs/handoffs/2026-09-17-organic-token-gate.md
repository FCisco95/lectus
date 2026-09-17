# Lectus - HANDOFF

## Metadata

- Last Updated: 2026-09-17 midday (Windows PC)
- Repository: `lectus` (github.com/FCisco95/lectus) — **public** since this morning
- Branch: `master` at `8193739`, pushed, clean
- Version in manifests: `0.6.0`, tag `v0.6.0` pushed
- Live Windows install: `%LocalAppData%\Lectus\chirp.exe` (gate build, 2026-09-17 12:05)

## TL;DR

The **Organic token gate** is built, tested live, and shipped. Lectus is free for
people holding **$20 of ORGANIC**; the wallet is linked once by signing a nonce,
and the balance is re-read every 12 h. No account, no subscription.

Earlier the same morning: the Home window landed (tray / shortcut / pill open it,
login silent via `--autostart`), and the repo was made public.

Specs: `docs/superpowers/specs/2026-09-17-organic-token-gate.md`,
`docs/superpowers/specs/2026-09-17-home-window-design.md`.

## What to do next

1. **CI run `35215811370` (v0.6.0) was in progress at handoff.** Check it:
   `gh run view 35215811370`. Note the v0.5.0 run failed at ~46 min on Windows
   CPU portability — if this one repeats that, read the logs before re-tagging.
2. **Publish the draft release.** The in-app updater fetches
   `releases/latest/download/latest.json`, and GitHub only serves `/latest/` for
   a *published* release — which is exactly why the updater 404s today. Publishing
   v0.6.0 makes every installed copy update itself on next launch.
3. **Delete the stale v0.5.0 draft** so nobody grabs a pre-gate build.
4. Unasked-for but pending decisions, all one-liners — see the gate spec:
   onboarding's wallet step is skippable; existing installs lock immediately on
   update (no trial window for `Unlinked`); public-RPC rate limits untested.

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

## Next-session prompt

```text
Lectus 2026-09-17 midday: Organic token gate shipped and verified live
(master 8193739, tag v0.6.0, CI run 35215811370 was still building). Read
docs/HANDOFF.md and docs/superpowers/specs/2026-09-17-organic-token-gate.md.
First: check that CI run, publish the draft release (the updater 404s until
a release is published), and delete the stale v0.5.0 draft. Do not reopen
mute or the capture pipeline.
```
