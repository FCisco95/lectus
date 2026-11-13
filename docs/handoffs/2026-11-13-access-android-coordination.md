# Lectus access and Android coordination

Updated: 2026-11-13. Four discovery reports received; no product design is approved.
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
Resume Lectus Run run_db09bc5a3181 as the central Orca coordinator. Initial discovery is complete; an expanded follow-up frontier is submitted. No account/billing/Android product design is approved.
Files: docs/HANDOFF.md, docs/handoffs/2026-11-13-orca-architect-prompt.md, docs/handoffs/2026-11-13-access-android-coordination.md.
Model: gpt-6-astra xhigh — cross-platform architecture and auth/billing supervision.
Skills: task-observer, model-router, orca-cli, orchestration, parallel-feature-development, brainstorming, grill-me.
Inspect the Run and current consumer before resuming it; process reports/questions, verify evidence and dispatch newly ready work without duplicating discovery. Present the consolidated design for approval before product implementation.
```

## Discovery received and verified premises

All four original Tasks reported succeeded through worker_done. Their deliverables
are reports only; no source code, tests or native acceptance were delivered.
The coordinator read all reports and directly checked these material premises:

- `src-tauri/src/license/mod.rs:139` preserves a seven-day local trial and gives
  an allowing holder verdict precedence. `:167` returns Active for a cached
  positive balance without checking its age; `license/commands.rs:206` preserves
  the cache after RPC failure. A seven-day holder receipt changes current behavior.
- `src-tauri/Cargo.toml:72` and `:87` configure inference for desktop targets;
  a mobile entry annotation does not prove Android support.
- `transcription/model.rs:175`, `ai/local_llm.rs:24` and `translation.rs:8` list
  about 3408 MB for Base+cleanup+translation downloads, not measured process RAM.
- The contracts report found no Lectus email-account/billing contract in its
  bounded search. AI-provider subscription auth is a different system.
- These are static checks. No historical suite was rerun or fresh runtime
  acceptance claimed; source is unchanged since the verified discovery base.

Android recommendation: native Kotlin input method plus reusable Rust core,
subject to a real-device prototype. A Tauri settings Activity remains an
alternative. SDK/NDK readiness was not demonstrated in the inspected Windows
locations; other disks/WSL hosts were not exhaustively inspected. ASR-only is a
possible disposable prototype, not approval to reduce the mobile product or
claim local cleanup/translation parity.

Primary sources directly checked by the coordinator:
- [Android input methods](https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method) supports the native service architecture; it does not prove Lectus microphone/insertion behavior.
- [Play Payments FAQ](https://support.google.com/googleplay/android-developer/answer/10281818?hl=en) permits consumption-only access and informational text without direct purchase links. [Payments policy](https://support.google.com/googleplay/android-developer/answer/9858738?hl=en) covers in-app purchases and payment-directed signup flows. Holder/custom-code acceptance remains unproved.
- [Lemon Squeezy prohibited products](https://docs.lemonsqueezy.com/help/getting-started/prohibited-products) lists crypto-related products; Lectus eligibility is undetermined.
- [Paddle AUP](https://www.paddle.com/help/start/account-verification/what-am-i-not-allowed-to-sell-on-paddle) lists virtual currency/stored value including gift cards/vouchers. One direct fetch failed; the official indexed page was retrieved, dated 13 April 2026. [Promotional guidance](https://www.paddle.com/resources/best-discounts-for-saas-companies-with-paddle) separately describes free trials, additional service months and discounts. This does not establish either a blanket ban or specific approval for Lectus free access codes.

Correction to the initial billing framing: the owner decided to offer codes of
1/3/12 months, not to sell gift cards. Paid gifting and prices are not approved
requirements. The policy follow-up must separate those models; no renaming to
evade a policy, hidden checkout or provider-approval claim is accepted.

Long offline receipts plus device replacement cannot promise immediate revocation
or strictly two physical devices in simultaneous offline use. The proposed
replacement throttle, holder TTL, mandatory holder email and clock-failure policy
have not been accepted on the owner's behalf.

## Historical follow-up startup blocker

Independent review depends on the four original completed Tasks. The policy
clarification reuses the billing Dispatch mailbox. Both new starts failed before
task execution; no review or clarification result exists:

| Task | Failed Dispatch | Exact retained terminal |
|---|---|---|
| `task_82059cb3e1e4` independent review | `ctx_541b91a82e71` | `term_a140ae58-d454-45af-aaa5-f985f060afce` |
| `task_5d630d994181` free-code clarification | `ctx_661b2bc64a9f` | `term_c388d2660c5ee62ce6143ec761bc9395` |

worker-show/worker-read proved both live at `agent-hooks-review-prompt`:
"5 hooks are new or changed"; the runtime warns hooks can run outside the sandbox.
The owner was asked to choose "Continue without trusting" in both tabs.
No answer is recorded. No hook trust, terminal steering or automatic retry occurred.
The terminals are retained for that requested action; neither is claimed exited.

After confirmation, inspect exact identities/readiness and follow the installed
recovery guide. Recover the same Tasks/attempt chain or reuse their proven ready
terminal through worker-start where supported; never send terminal input or
create replacement Tasks. Release only an exact owned failed terminal being
replaced, following worker-release receipts. No broad cleanup or resets.

Requested models: independent critical review Astra xhigh; policy follow-up
reuses Sol high. Actual token/cost totals are unavailable in the launch receipts.
Four original reports remain completed despite these two failed follow-up starts.

## Pending owner decision

Android distribution/purchase question is pending: Play with in-app subscription
(coordinator recommendation for non-crypto UX, with extra integration), Play
consumption-only plus website subscription, or APK first. No answer is recorded.
The hook prompt is a separate operational action, not product approval.

After the prompts: finish evidence review, close the offline/device tradeoff and
other material branches, write the full design, obtain its independent review and
owner approval, then freeze contracts and dispatch implementation. No complete
design or implementation plan has been represented as approved.
## Latest: expanded independent frontier

The owner explicitly requested more aggressive agent parallelism. The coordinator
expanded work to seven follow-up assignments while keeping production implementation
behind design approval. The earlier hook menus were no longer present: both exact
retained sessions showed Ready. No coordinator hook-trust change or terminal input
was used; the existing Tasks were recovered through worker-start.

| Existing Task | Current Dispatch | Launch evidence |
|---|---|---|
| `task_82059cb3e1e4` review | `ctx_76e92f87d028` | ready; retained Astra xhigh session |
| `task_5d630d994181` policy | `ctx_10a469b72615` | input accepted; turn_start_unobserved/outcome_unknown, retained Sol high |

| New lane / Task | Dispatch | Model / effort | Exact ownership |
|---|---|---|---|
| Auth `task_6e09f5a20133` | `ctx_ee7eecf17a88` | Astra high | `auth-lab/` |
| Receipts `task_7fdeab05b636` | `ctx_4cdba1e87c90` | Sol high | `receipt-lab/` |
| Gift/event races `task_0e7efc83bd09` | `ctx_74113e23eac0` | Sol high | `grant-lab/` |
| Android interaction `task_2e945f88dead` | `ctx_760322cbe728` | Terra high | `android-ui/` |
| Tooling inventory `task_e4b6bff6b4ea` | `ctx_e8f1c5ee72fd` | Luna medium | `toolchain-inventory/` |

Owned directories are under `.context/lectus-access-android-20261113/`.
Briefs/receipts remain coordinator-owned. No product source, shared contracts,
manifests or lockfiles are assigned to these lanes. Auth/receipts depend on completed
contracts+lifecycle discovery; grants on billing+lifecycle; Android UI on
Android+contracts; tooling on Android discovery. All dependencies were ready.

Receipt/grant labs must produce runnable synthetic adversarial tests with negative
controls. Auth may produce a local handoff/PKCE state-machine probe. Android UI is
an interactive browser simulation with screenshots, not a native Android build.
Tooling is narrow read-only inventory beyond the Windows locations already inspected.
Only existing tools/private outputs are permitted; no downloads, hardware, external
accounts, actual charges or messages. This separates experiment evidence from
approved production contracts and real platform acceptance.

The four new non-auth launches were ready; auth returned outcome_unknown after input
acceptance. Together with the recovered workers: five confirmed starts and two accepted
inputs whose turn start was not observed. Follow the receipt: do not resend/abandon
from silence; yield, process wake messages, and inspect the exact Dispatch if nothing
arrives. These are launch states, not completion claims.

Resource evidence at expansion: about 12.9 GiB free RAM; lightweight Node/Python
experiments plus one browser prototype, with no native builds/shared targets.
Model routing follows the manifest's per-lane rationale; cost/usage remains unavailable.

A first inline command for writing briefs was refused by the WSL PreToolUse guard
because it matched command-like prose inside the brief. It performed no task creation.
The coordinator then wrote the data with a structured patch and used short CLI commands
reading spec files; the guard was not disabled. Exact task and launch receipts are
retained in each owned directory and expansion-tasks.json.

The coordinator also wrote `docs/designs/2026-11-13-access-and-android.md` as an
explicitly incomplete engineering draft to collect these results. It is not a final
approval packet or frozen contract. The outstanding Android purchase-channel question
was not answered by the request for more parallelism. Continue material questions one
at a time while these independent assignments run.