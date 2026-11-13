# Lectus - handoff

Updated: 2026-11-13. Branch: feat/remote-dictation. No push, tag or installation.

## Active work: access model and Android planning

The setup-wizard implementation is complete. A new product-design conversation is
in progress. No account, billing, voucher or Android code has been implemented.
Next: choose Android distribution, then finish the unresolved access-flow decisions
and obtain approval of the written design before implementation.

## Confirmed owner decisions and facts

- The owner has no registered company. Individual publishing is a provisional
  preference for a personal portfolio/CV; public legal identity remains unset.
- The owner created Organic and Mycel and describes both as public fairlaunches
  starting at zero on launchpads, traded on DEXs. This does not establish a legal
  classification, a free-offer exemption or compliance with MiCA.
- Lectus is a benefit for holders/community contributors, with a proposed normal
  subscription around EUR 5/month for users who do not want cryptocurrency.
- The paid price covers local AI, not publisher-funded cloud inference. Existing
  bring-your-own-API functionality stays available; provider usage belongs to the
  user's provider account and needs explicit processing consent.
- Gift codes offer 1, 3 or 12 months to friends/other people.
- Paid/gift access uses an email account without a crypto wallet. Passwordless email
  was proposed and accepted; Wispr Flow is the reference for browser sign-in and
  account recovery across devices, not proof that it uses the same auth method.
- Up to two personal devices per account, including computer plus phone.
- After activation, local use should work offline for the paid or gifted period.
- Android only for the first mobile version. iPhone is outside the current scope.

## Proposed defaults, not separate owner decisions

Same features for each valid access route. Unique one-use gift codes, duration
starting on redemption, no bank card needed for a gift, no automatic charge when
it expires. A normal subscription or holder eligibility can continue access.
Retain the existing seven-day trial unless the new design explicitly changes it.

## Open design branches and feasibility gates

- Android distribution: Google Play as primary channel, direct APK beta, or another
  combination. This affects purchase UI, billing and review requirements.
- Trial sign-in timing, account/wallet association and account recovery details.
- Device replacement/revocation while old devices remain offline; a hard immediate
  two-device limit cannot be assumed alongside long offline receipts.
- Payment-provider eligibility, taxes/final consumer price and token-related
  business review. No merchant, hosting or authentication service was activated.
- Android dictation integration and local-model performance need a real-device
  prototype. The current desktop Tauri implementation is not an Android build.

Google Play permits consumption-only apps to use existing access bought elsewhere;
in-app digital purchases have billing requirements and regional/program exceptions.
This does not pre-approve our holder/voucher model or allow hidden review behavior.
Source: https://support.google.com/googleplay/android-developer/answer/10281818
Blockchain policy: https://support.google.com/googleplay/android-developer/answer/13607354
Apple has separate crypto/license unlocking restrictions; iOS was parked, not solved.

## Completed source and verification

Setup wizard, seven-day trial, sound-alikes/prompt budget, bounded local folder scan
and branded NSIS art are complete through ca23c01; authorized review fixes: 061a023.
Last full checks: 310 native passed, 0 failed, 13 ignored; frontend build passed;
25 wizard scenarios, 11 guided scenarios and 18 legal tests passed. UI IPC is mocked.
No physical microphone unplug/permissions, fresh install or installer acceptance
was performed. macOS is uncompiled; real Windows-to-Mac insertion remains pending.
Detailed implementation history and review scope are preserved in
docs/handoffs/archive/HANDOFF-2026-11-13-setup-complete.md.
Generated smoke output was removed; unrelated .context/reviews/diagnostics preserved.

## Release and earlier acceptance still pending

legal/manifest.json is unchanged/unapproved. Final publisher details, licence/support
terms, identity disclosures and third-party obligations are unresolved. Generic
approval of product rules does not fill missing facts or approve publication.
The microphone privacy sentence needs to include shortcut practice/permission tests
when the legal drafts are finalized. No legal publication flags were changed.
Record four owner WAVs and run the bias benchmark before changing prompt wording.
English-sentence myself/Mycel ambiguity and automatic word learning remain separate.
Official ChatGPT/Claude login and real inference are still pending; historical
candidate receipts predate the wizard. A fresh build/readiness is needed for live tests.

## Artifacts and suggested skills

Canonical art: src-tauri/icons/installer-header.bmp and installer-sidebar.bmp;
generator: scripts/gen_installer_art.ps1. No new keys, accounts or deployed services.
Use brainstorming and grill-me for the active design; writing-plans only after
approval. Use verify/computer-use only for separately authorized native acceptance.

## Next-session prompt

```text
Act as the central Lectus architect and Orca coordinator. Read and follow docs/handoffs/2026-11-13-orca-architect-prompt.md and docs/HANDOFF.md, then inspect the current state and dispatch the independent discovery frontier. Do not repeat completed wizard work or infer that a billing/mobile implementation design is already approved.
Model: gpt-6-astra xhigh (cross-platform architecture, authentication and billing supervision).
Skills: task-observer, model-router, orca-cli, orchestration, parallel-feature-development; brainstorming/grill-me for material gaps.
```
