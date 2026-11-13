# Subscription AI Implementation Plan

> For agentic workers: use superpowers:subagent-driven-development.

**Goal:** Connect login-backed subscriptions and route existing text AI features correctly.
**Architecture:** Rust subscriptions module owns account lifecycle and text requests; shared ai/translation dispatch uses it; React settings exposes connections and selectors.
**Tech Stack:** Tauri 2, Rust, React/TypeScript; official OAuth and Claude CLI.
**Spec:** ../specs/2026-10-22-subscription-ai-design.md

## Global Constraints
Preserve current dirty feature branch. No commits of pre-existing work. Baseline snapshot under .context/subscription-ai/baseline. No capture/injection/shortcut changes; no installation/publication. Secrets only in native store/official CLI. No billing fallback. HTTP API remains offline. User requested execution; no additional design permission gate.

## Review Focus
1. Login cancel/disconnect racing with refresh/generation cannot resurrect account or release stale output.
2. Partial streams, CLI failure, output overflow or auth mismatch return errors without exposing secrets.
3. Global cleanup off + profile on works; revoked permission or changed selection stops upload.
4. CLI transcript cannot access tools, user/project hooks, MCP or unrelated files.
5. UI reflects actual backend account state and stale async results do not overwrite newer choices.

## Task 1: Subscription service
Owner: src-tauri/src/subscriptions/**, Cargo.toml/Cargo.lock.
Interfaces: generate(provider: &str, text: &str, instructions: &str, model: &str) -> anyhow::Result<String>.
Tauri commands: get_ai_connections() -> Vec<ConnectionStatus>; start_ai_login(provider: String) -> Result<(), String> opens system browser via backend; cancel_ai_login(provider); disconnect_ai(provider); list_subscription_models(provider) -> Vec<SubscriptionModel>; test_subscription(provider, model: String) -> TestResult.
ConnectionStatus {provider,state,account_label: Option<String>,message}; state disconnected/connecting/connected/unavailable/error. SubscriptionModel {id,name}. TestResult {text,elapsed_ms}. All commands async/blocking-safe. Module path derives from Config::config_file().parent()/subscriptions. Claude official CLI uses isolated CLAUDE_CONFIG_DIR. ChatGPT native credential store.
- [x] Implement contract and focused red/green tests, fetch official protocol docs before coding.
- [x] Compile and review security/state handling.

## Task 2: Existing feature integration
Owner: src-tauri/src/{lib.rs,config.rs,ai/mod.rs,translation.rs}, routing tests as needed.
Config additions: chatgpt_model:String default empty, claude_model:String default empty (official default), chatgpt_processing_acknowledged:bool default false. Existing claude_processing_acknowledged retained. Cleanup engine chatgpt/claude; translation engine chatgpt/claude plus existing local/cloud.
- [x] Tests for migration and provider/consent routing, profile enablement and revoked permissions.
- [x] Register commands, route cleanup and translation to Task 1, pt-PT prompt/term preservation, refresh effective-profile authority.
- [x] Verify native and remote entrypoints, preserve local HTTP API offline contract.

## Task 3: Settings integration
Owner: src/components/settings/{AIConnections.tsx,AIPanel.tsx,DictationPanel.tsx,types.ts,HomePanel.tsx,HelpPanel.tsx}, related UI tests only.
- [x] Implement connection cards with login/cancel/disconnect/check/test and clear subscription usage copy. No keys/password/token entry for subscriptions.
- [x] Select real ChatGPT catalog, Claude default/official aliases, separate cleanup/translation engine; common per-provider model selection.
- [x] Add consent and errors, no auto-enable on sign-in, preserve defaults and existing API-key settings.
- [x] Typecheck/build; cover connection/selection/error states with UI harness.

## Task 4: Integrated verification and review
- [x] Native tests and frontend build; review final task diff against baseline.
- [ ] Fixes complete; live provider checks still require explicit human login, no global auth reuse.
- [x] Record prepared/tested/live status, refresh HANDOFF without losing older unresolved hardware and release tasks.

## Reviewed interface amendment
Task1 additionally exports inference_epoch()->u64, invalidate_inference(), generate_guarded(provider,text,instructions,model,expected_epoch:u64)->Result<String>. Task2 captures expected epoch under the current Config lock during authorization and passes it through guarded cleanup/translation dispatch; relevant save_config updates invalidate before releasing lock. This is required by Review Focus revocation-before-upload; it must not cancel login flows.

## Final implementation decisions
Claude is personal Pro/Max only and rejects detected managed policy sources; enterprise/server-managed policy is not overridden. Source and automated verification complete; login-backed live acceptance pending. See ../../reviews/2026-10-22-subscription-ai.md and receipt.
