# Lectus access and Android coordination

Updated: 2026-11-13. Discovery is in progress; no product design is approved.
Next: consume this Run's reports, verify consequential claims, consolidate the
design and close one material owner decision at a time before design approval.

## Run and verified baseline

- Run: `run_db09bc5a3181`; Orca CLI/runtime guide: 1.4.257.
- Branch: `feat/remote-dictation`; source base: `3b1736590c34efa11a0858acaf98949a8c4d9147`.
- Coordinator at kickoff: `term_9569fa1f-391f-4a51-8a56-04826a2a6584`.
  This is a runtime handle, not a portable identity. Resume the Run through the
  installed orchestration guide and verify its current consumer after restart.
- Existing untracked `.context/`, `docs/reviews/` and `scripts/diagnostics/`
  were present before this Run and remain preserved. Tracked source was clean.
- No earlier matching Run was found in the complete 17-Run inventory.

## Discovery ownership and DAG

All four tasks are independent roots. Their consolidated evidence precedes the
design review and owner approval. Implementation tasks must be created only
after approval, with frozen shared contracts and explicit integration ownership.

| Lane | Orca Task / Dispatch | Model / effort | Owned report |
|---|---|---|---|
| Existing contracts | `task_c849944631c6` / `ctx_507d4670097b` | `gpt-6-luna` / medium | `contracts.md` |
| Android feasibility | `task_55a39693bbec` / `ctx_b96e511ffffd` | `gpt-6-astra` / high | `android.md` |
| Billing/store constraints | `task_2844b285c318` / `ctx_a91aaddb7ede` | `gpt-6.1-sol` / high | `billing-policy.md` |
| Access lifecycle | `task_12280bf6749b` / `ctx_b65e0b236e25` | `gpt-6-astra` / high | `lifecycle.md` |

Reports are beneath `.context/lectus-access-android-20261113/`, ignored locally
through `.git/info/exclude`. Android alone may use `android-spike/` there for a
small disposable synthetic probe with installed tools. No source edits, commits,
dependency installation, real hardware tests or heavy/shared builds are delegated.

Routing rationale: Luna only extracts factual source references; Sol researches
current primary policy/provider sources; Astra handles native architecture and
access/offline invariants. The coordinator session was retained as requested;
its exact model variant/effort is not exposed in the system identity.

All workers use the same read-only source checkout and distinct report paths.
They must report through `worker_done`, ask the coordinator through Orca `ask`,
preserve others' work and never spawn teams. The coordinator alone owns shared
contracts, source integration, manifests, lockfiles and `docs/HANDOFF.md`.

## Decisions and boundaries

Confirmed product decisions and proposed defaults remain in `../HANDOFF.md`.
The full authorized workflow is `2026-11-13-orca-architect-prompt.md`.
Play primary plus APK beta is a proposal, not a settled distribution decision.
Long offline validity versus immediate revocation/device replacement must be
resolved explicitly; no hard immediate two-device guarantee is assumed.

No push, tag, merge to main, installed-app replacement, live charge, service
activation, production deployment or change to `legal/manifest.json` is authorized.
No real capture/injection or shortcut changes are approved by discovery.
The historical wizard/native test counts in the canonical handoff are retained
as historical evidence. No new product tests or native acceptance are claimed.
Actual worker token/cost usage is unavailable in the launch receipts.

## Generated artifacts

| Artifact | Location | Durability |
|---|---|---|
| Run, Tasks and Dispatch records | Orca Run above | Local Orca state; inspect before reuse |
| Briefs, launch receipts and reports | `.context/lectus-access-android-20261113/` | Local ignored evidence, not machine-portable |
| Coordination state | This file and `../HANDOFF.md` | Repository handoff |

No credentials, accounts, deployments or scheduled jobs were created.
When reports settle, summarize decisive evidence in the portable handoff before
removing only this Run's disposable output. Preserve unrelated generated folders.

## Suggested skills

Use task-observer, model-router, orca-cli, orchestration and
parallel-feature-development; brainstorming/grill-me for unresolved design
branches. Use writing-plans only after the complete design is approved.

## Next-session prompt

```text
Resume Lectus Run run_db09bc5a3181 as the central Orca coordinator. Discovery is in progress; no account/billing/Android product design has been approved.
Files: docs/HANDOFF.md, docs/handoffs/2026-11-13-orca-architect-prompt.md, docs/handoffs/2026-11-13-access-android-coordination.md.
Model: gpt-6-astra xhigh — cross-platform architecture and auth/billing supervision.
Skills: task-observer, model-router, orca-cli, orchestration, parallel-feature-development, brainstorming, grill-me.
Inspect the Run and current consumer before resuming it; process reports/questions, verify evidence and dispatch newly ready work without duplicating discovery. Present the consolidated design for approval before product implementation.
```
