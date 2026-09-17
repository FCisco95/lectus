# Organic token gate — Lectus

**Date:** 2026-09-17
**Status:** implemented (not yet live-tested)

## What this is

Lectus is free for people who hold ORGANIC. It is a **membership check, not a
subscription**: no account, no Stripe, no word meter, no card on file.
Transcription runs locally, so there is nothing metered to bill for.

It is also **not DRM**. The repo is public and a `.exe` can be patched. The gate
exists so "Organic holders get Lectus" is true and casual sharing does not
happen — not to defeat a determined attacker. A harder gate, if ever wanted,
belongs on downloads/updates, not on every keystroke.

## The rules

| Thing | Value |
|---|---|
| Mint | `DuXugm4oTXrGDopgxgudyhboaf6uUg1GVbJ6jk6qbonk` (ORG, 6 decimals, immutable) |
| Floor | **$20 of ORGANIC** — dollars, not a token count |
| Grace | 7 days below the floor (or unreachable chain) before dictation locks |
| Re-check | every 12 h in the background, plus a manual "Check again" |
| Balance source | `getTokenAccountsByOwner`, public mainnet RPC, `LECTUS_RPC_URL` to override |
| Price source | Jupiter `lite-api.jup.ag/price/v3` |

The floor is USD-denominated, so the token count is derived at every check from
the live price (≈ 7,930 ORG at $0.00252 on 2026-09-17). **Consequence worth
knowing:** ORGANIC liquidity is thin (~$29k) and moved +68% in the 24 h before
this was built. A pump halves the tokens needed; a dump doubles them. Switching
to a frozen token count is a one-line change to `FLOOR_USD` / `status_of`.

No API key ships in the binary — the repo is public, so any embedded key would
leak with every download.

## The flow

1. **Link (once).** A Tauri webview cannot see browser extensions, so the app
   serves a page on `127.0.0.1:<random port>` and opens the real browser, where
   Phantom/Solflare/Backpack live. The page asks the wallet to sign a nonce.
2. **Prove.** Rust verifies the ed25519 signature over that nonce
   (`verify_strict`). The nonce is single-use. No transfer, no spend, no approval.
3. **Check.** Sum the wallet's ORGANIC across its token accounts, multiply by
   the Jupiter price, compare to $20.
4. **Persist.** `license.json` in the app data dir, beside `history.json`, written
   atomically. Deliberately *not* in `config.json`: the Settings webview
   round-trips the whole config, and a stale snapshot must never clobber the link.
5. **Live.** Re-check on launch when the cadence is due. The wallet never pops
   up again.

## Failure behaviour

Every failure fails *open*, inside the grace window:

- Grace is measured from `last_ok_ms` — the last time the wallet was **seen**
  above the floor. A failed read leaves it untouched, so an RPC outage or a
  flight cannot shorten it.
- An RPC error is an error, never "balance 0". A zero or negative price is
  rejected outright, since it would let any balance clear a dollar floor.
- A wallet that links while the chain is unreachable is still linked; the
  balance read retries in the background.
- The gate only ever **declines to start** a dictation. It cannot interrupt one
  in flight, and it is checked ahead of the hold chime and the audio ducker, so
  a refusal never mutes the user's music.
- A wallet that has *never* been seen above the floor gets no grace.

## Where it lives

| File | Role |
|---|---|
| `src-tauri/src/license/mod.rs` | `License`, `Status`, grace/floor logic, persistence |
| `src-tauri/src/license/verify.rs` | nonce + ed25519 proof |
| `src-tauri/src/license/chain.rs` | balance (RPC) and price (Jupiter) |
| `src-tauri/src/license/connect.rs` | loopback server + the connect page |
| `src-tauri/src/license/commands.rs` | Tauri commands, background re-check |
| `src-tauri/src/lib.rs` | `gate_blocks_dictation` at both pipeline entries |
| `src/components/settings/MembershipPanel.tsx` | link / status / unlink UI |
| `src/components/Onboarding.tsx` | wallet step (skippable — see below) |

## Open decisions

1. **Onboarding is skippable.** The wallet step has a "Skip for now" button; a
   skipper reaches the app and the first hotkey press opens Membership instead
   of dictating. Making it mandatory is a one-line change.
2. **Existing installs lock immediately** on updating to this build, since no
   wallet is linked. If that is too abrupt, give `Unlinked` a trial window in
   `status_of`.
3. **Public RPC rate limits** are untested under real use. `LECTUS_RPC_URL`
   exists as the escape hatch.
