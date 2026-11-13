THE JOB
Act as the Lectus architect, integration owner and central Orca coordinator. Maximize useful parallel progress and speed while controlling total token cost, rework and resource contention. Every worker reports to you. This is supervised Orca orchestration, not a handoff where you leave.

STARTUP
Read AGENTS.md and docs/HANDOFF.md. Load task-observer, model-router, orca-cli, orchestration and parallel-feature-development. Use brainstorming/grill-me only to close material gaps in the new design; do not restart settled decisions. Resolve the session's Orca executable once and load its version-matched guides. Before DAG/worktree actions, load orchestration references/coordinator-loop.md and references/placement-and-remote.md. Use --help for unsupported/unclear flags. Never invent commands or silently substitute built-in subagents for Orca workers.

Inspect git status, branch, HEAD and existing Orca Runs/workspaces before creating anything. Preserve unrelated dirty work. Reuse the Run for this objective if one exists; otherwise create one and record its Run ID. Pass explicit Run/Dispatch identities where needed so reports return to this coordinator, including after a restart.

PRODUCT CONTEXT
The desktop setup wizard, trial, vocabulary/sound-alikes and NSIS art are already implemented. Last full checks recorded in the handoff: 310 native passes, 13 ignored; 25 wizard scenarios, 11 guided scenarios and 18 legal tests. UI uses mocked IPC. Do not rebuild or repeat unchanged completed work.

New direction to design and implement after design approval:
- Local AI access at a target of about EUR 5/month. No publisher-funded cloud inference. Existing bring-your-own-API cloud options remain, with provider costs charged to the user's provider account and explicit processing permission.
- Community/holder access remains an alternative to payment; non-crypto users need no wallet.
- Gift codes grant 1, 3 or 12 months. Proposed defaults: one use, starts on redemption, no card required, no automatic charge when a gift ends. Distinguish confirmed decisions from these proposed defaults.
- Passwordless email account for paid/gift access and recovery across devices. Wallet is for holder verification. Wispr Flow is a UX reference, not an instruction to clone its implementation or assume it uses magic links.
- Up to two personal devices, including computer plus phone. After activation, local use should work offline during the paid/gift period. Resolve the interaction between long offline validity, revocation and device replacement explicitly.
- Android only for the first mobile version; no iOS implementation. Account/licensing should be shared with desktop. Android dictation integration and local-model performance need their own evidence.
- Google Play primary distribution with APK beta testing is the proposed direction in the handoff. Validate applicable billing/blockchain policies before committing to checkout behavior; do not infer policy approval or hide functionality from review.
- Retain the seven-day trial unless an explicit design decision changes it.

The owner has no registered company and prefers individual publishing provisionally. The owner created Organic and Mycel and describes public fairlaunches starting at zero, traded on DEXs. Do not infer a regulatory exemption, final legal identity or publication approval. legal/manifest.json remains unapproved and must not be changed in this work without separate explicit authorization.

MODEL AND EFFORT AUTHORIZATION
I explicitly authorize choosing the following worker models and effort through Orca, using the current model-router matrix:
- gpt-6-astra, high/xhigh: architecture, cross-cutting invariants, difficult native/Android integration, critical review and problems where a silent error would be expensive.
- gpt-6.1-sol, high: clearly specified implementation and meaningful tests; xhigh for difficult auth, entitlement or billing work when appropriate.
- gpt-6.1-terra: visual inspection, UI exploration and suitable computer-use tasks when its capabilities fit.
- gpt-6-luna: bounded read-only extraction/inventory and mechanical summaries; not auth, payments, security decisions or final critical review.
Use supported alternatives from model-router if necessary, recording the reason. Do not choose a weak model merely to save per-token cost. Escalate after a costly failed attempt or uncertain correctness, rather than paying for repeated blind rewrites. Do not use maximum effort everywhere. Keep this coordinator on its explicitly selected model; do not silently replace its session.

PARALLEL EXECUTION
Build a dependency DAG with bounded tasks, acceptance evidence and an explicit owner for each file/module. Dispatch every ready independent task, not one task followed by an unnecessary wait. Optimize the critical path: contracts and shared foundations must unblock implementation lanes early. Match concurrency to actual Orca capacity, independent work and CPU/RAM; do not launch redundant agents for appearance.

Begin with parallel discovery lanes covering: existing contracts/integration points; Android IME/local-inference feasibility and tooling; billing/store/provider constraints; and UX/access lifecycle including gifts, offline use and device changes. Consolidate evidence before asking only the highest-impact unresolved user question. Continue independent work while waiting. Routine technical choices are yours; record their tradeoffs. Present a concise complete design for approval before product implementation. I authorize isolated local disposable technical spikes with synthetic data where they answer a specific uncertainty; they are not approved production code or permission to activate external services.

After design approval, freeze shared API/DTO/state-machine contracts, create the implementation DAG and assign non-overlapping lanes. Shared types, schemas, lockfiles, manifests and integration entry points have one owner. Workers request changes from that owner. Use Orca child worktrees for conflicting/independent implementation, explicitly based on the verified integration HEAD, not an accidental repository-default branch. No raw git worktree creation behind Orca. Only you integrate worker commits and resolve cross-lane conflicts. Workers must not spawn their own teams or edit outside their ownership.

ORCA LIFECYCLE
Use the documented run-create/task-create --deps/task-list --ready/worker-start workflow. Set --model and --effort explicitly for new launches; worker-start supports --spec-file and --base-branch for new worktrees, subject to its current help. A Task needs a Run. Do not combine mutually exclusive task/spec inputs.

Every brief includes THE JOB and DONE MEANS, Task/Dispatch identity, objective, exact ownership, frozen contracts, dependencies, constraints and required proof. Tell workers they share the project with others and must preserve other work. Require worker_done through their injected Orca preamble. Workers ask you, not the user, through Orca ask; you reply when the answer follows from existing decisions. Use orchestration send/reply for steering, never a worker's terminal.

On each wake, process reports/questions, inspect the evidence, integrate eligible results and immediately dispatch the newly ready frontier. Reuse a settled worker for a related follow-up through its Dispatch mailbox. Follow receipt.next. Start all ready independent workers in the turn, then yield when you have no useful coordinator work; Orca wakes you on results. Do not poll, run watch loops or consume mail in background scripts. A yield is not abandonment of coordination.

For failed/unknown launches, inspect failedStage and the documented recovery reference. Preserve request/operation IDs and replay the same accepted request when supported; never launch duplicates because output was silent. Never infer exit from a timeout. Release only workers/resources owned by this Run after their work is secured, following Orca's recovery/cleanup rules.

TOKEN EFFICIENCY
Send a small self-contained brief plus relevant contract/file paths and base SHA, not the whole conversation. Do not send everyone to read the entire repository or repeat discovery another lane already owns. Reports should be compact: outcome, changed paths/commit, commands/results, unresolved issues and next dependency. Keep long logs and receipts in per-task ignored locations; report paths and summaries. Reuse valid scoped evidence, rerunning for changes, gaps, contradictions or required final integration checks. Track actual usage/cost if the runtime exposes it; otherwise label it unavailable rather than inventing totals.

TESTING AND RESOURCE ISOLATION
Use test-first development for behavior changes, observe the intended failure, make it pass and run appropriate checks. Include real integration proof where available; mocked IPC, reachable model catalogs, an accepted login prompt or an email-send response are not proof of native capture, inference, entitlement or email delivery.

Native Windows caution: test-release.cmd hard-codes the canonical repo cwd and C:\lt target junction. A worker must never claim it tested its worktree by accidentally testing the coordinator checkout. Serialize canonical native runs on the correct integrated source. If isolated native checks are needed, use the same verified toolchain with an explicit candidate cwd and private target, without repointing C:\lt or changing global environment. Avoid simultaneous heavy builds against shared targets. Give parallel UI tests unique ports/output folders and isolated browser contexts. Never kill processes by executable name.

Use synthetic profiles and test/sandbox billing. Test signed access, identity binding, webhook authenticity/idempotency, gift redemption races, expiry/cancellation, device limits/offline semantics and coexistence of valid access routes. Inspect UI screenshots yourself. Include independent review for the integrated design and security-sensitive changes, prioritizing meaningful component review rather than a fresh expensive reviewer for every trivial edit.

BOUNDARIES
Preserve feat/remote-dictation as the coordination branch unless I explicitly authorize otherwise; workers may use child branches. Commit by explicit paths with one-line messages. Never push, tag, merge into main, run build-release.cmd/deploy-local.cmd, replace the installed app, or alter real user configuration. Preserve unrelated .context/reviews/diagnostics. Do not start real microphone/hardware tests without readiness. No live charges, external messages to customers/friends, paid hosting, production deployment or new external account/service activation without specific authorization after a concrete reviewable result exists. Never request or print secrets; keep them out of prompts, logs and Git. Keep audio, transcripts, history and user API keys out of the account/billing backend unless a separately approved design explicitly changes that.

DONE MEANS
The approved scope is integrated and reviewed, relevant tests pass on the actual candidate, and evidence clearly separates implemented, verified, mocked, native-accepted and published states. Keep an authoritative DAG/ownership/status ledger in Orca and a portable project handoff. On a genuine blocker, preserve work, report the exact evidence and continue unaffected tasks. Do not fake completion or stop at a plan when authorized work remains.

Communicate with me in concise Portuguese: completed result, current blocker, next concrete step. Ask one material question at a time. Finish with integrated commits, tests/evidence, model/effort choices and measured usage if available, remaining release gates, and the one next thing needed from me.

Start now by inspecting the existing state and launching the independent discovery frontier through Orca.
