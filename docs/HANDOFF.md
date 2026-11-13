# Lectus - handoff

Updated: 2026-11-13. Branch: feat/remote-dictation. No push/tag/installation.

## Active work

Run `run_db09bc5a3181`. Initial four discovery reports are complete and read.
The owner requested more aggressive parallel execution. Seven follow-up assignments
are submitted: review, policy, auth, receipts, grant races, Android UI and tooling.
Five launches are confirmed; auth and policy returned `outcome_unknown` after
input acceptance. Do not resend or infer failure; follow their Orca receipts.
Earlier hook menus cleared: both retained sessions were observed Ready before
reuse. The coordinator did not trust hooks or send worker-terminal input.
Next: consume reports, verify results, inspect the Android screenshots, and dispatch
newly ready independent work. Complete design/review/owner approval precedes product code.

## Authoritative pointers

- `docs/handoffs/2026-11-13-orca-architect-prompt.md`: authorization and boundaries.
- `docs/handoffs/2026-11-13-access-android-coordination.md`: DAG, identities and evidence.
- `docs/designs/2026-11-13-access-and-android.md`: engineering draft, NOT approval packet.
- `.context/lectus-access-android-20261113/`: ignored reports/labs/receipts; not portable.
  Initial source base `3b17365`; subsequent coordinator commits only change docs.

## Settled product requirements

- Local AI target about EUR 5/month; no publisher-funded inference. Existing BYO
  providers stay with user billing and explicit processing consent.
- Paid/gift access uses passwordless email without requiring crypto. Gifts grant
  1/3/12 months to friends/others; selling gift cards was not decided.
- Community/holder access remains an alternative. Two personal devices can include
  desktop+Android; paid/gift access works offline for the activated period.
- Android first; iOS excluded. Wispr Flow is a browser-login/recovery UX reference,
  not evidence of its implementation or permission to clone it.
- No registered company; individual publishing provisional; public identity unset.
  Owner describes Organic/Mycel fairlaunches traded on DEXs, not a legal classification
  or proof of any MiCA/free-offer exemption.

## Proposals and pending decisions

Proposed defaults: equal features across access routes; one-use gifts start on
redemption, need no card and never autocharge; retain the seven-day trial.
Pending owner question: Play with in-app subscription (coordinator recommendation),
Play consumption-only plus website subscription, or APK first. No answer recorded.
Still material: long offline validity versus replacement/strict two-device use;
holder identity/freshness migration, trial/recovery, time uncertainty and gift overlap.
A positive holder cache currently has no age-based expiry; seven days would change behavior.
Android requires native IME/device proof; local cleanup/translation parity is unproved.
Provider eligibility, jurisdiction/taxes/final price and publication identity remain open.

## Existing source and acceptance

Wizard, trial, sound-alikes/prompt budget, bounded scan and NSIS art are complete
through ca23c01; review fixes 061a023. Historical checks: 310 native passes/13 ignored;
frontend build, 25 wizard, 11 guided and 18 legal tests. UI IPC is mocked.
No fresh installer/microphone-permission acceptance; macOS uncompiled and real
Windows-to-Mac insertion pending. Discovery did not repeat unchanged full suites.
History/art paths: `docs/handoffs/archive/HANDOFF-2026-11-13-setup-complete.md`.

## Release gates

`legal/manifest.json` is unchanged/unapproved. Publisher, licence/support, identity
disclosures and third-party obligations remain unresolved. Privacy wording must
include shortcut practice/permission tests when legal drafts are finalized.
Four owner WAVs and bias benchmark precede prompt changes; myself/Mycel ambiguity
and automatic word learning remain separate. Official ChatGPT/Claude login and
real inference still need a fresh candidate/readiness check and native acceptance.
No capture/injection/shortcut change or installed-app replacement is authorized.

## Artifacts and suggested skills

Only docs plus ignored disposable synthetic labs are authorized in this phase.
No keys/accounts/services/deployments created by the coordinator. Usage totals unavailable.
Use task-observer, model-router, orca-cli, orchestration, parallel-feature-development;
brainstorming/grill-me for material gaps. Writing-plans only after full design approval.

## Next-session prompt

```text
Resume Lectus Run run_db09bc5a3181. Read docs/HANDOFF.md, docs/handoffs/2026-11-13-orca-architect-prompt.md, docs/handoffs/2026-11-13-access-android-coordination.md and docs/designs/2026-11-13-access-and-android.md. Initial discovery is complete; seven follow-up assignments were submitted. Inspect existing identities, handle reports/questions and dispatch the ready independent frontier without duplicating unknown launches. The Android-channel question and full design approval remain pending.
Model: gpt-6-astra xhigh — cross-platform architecture and auth/billing supervision.
Skills: task-observer, model-router, orca-cli, orchestration, parallel-feature-development, brainstorming, grill-me.
```