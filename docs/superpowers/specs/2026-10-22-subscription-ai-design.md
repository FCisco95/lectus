# Subscription AI for Lectus

User authorized execution on 2026-10-22 after discussing login-backed ChatGPT and Claude subscriptions, not API-key billing. Implementation target is this local desktop app; commercial distribution eligibility remains a separate release concern.

## Behavior
- Connect ChatGPT through the official Sign in with ChatGPT dynamic registration, OAuth PKCE loopback flow and eligible Responses endpoint. Keep tokens out of Config, frontend and logs; use native credential storage. Validate identity and granted scopes, rotate refresh credentials safely. No undocumented backend-api or imported Codex credentials.
- Connect Claude through the installed, unmodified official Claude Code login, in a Lectus-specific configuration directory. Never collect or proxy Claude credentials. Require subscription authentication before text generation; strip API/provider overrides from child environment. No silent paid API fallback; subscription usage remains subject to provider limits and external extra-usage settings.
- AI settings show connection state, login/cancel/disconnect, model choice and a synthetic test action. Both providers available for cleanup and explicit translation. Preserve existing local and API-key choices and defaults.
- Only text goes to subscription models. Speech recognition stays local/Groq. Tone and per-app enablement remain wired, Portuguese cleanup uses pt-PT. Preserve names, numbers, negations, URLs and dictionary spellings; transcript instructions are data, never agent commands.
- Provider operations have timeouts, size limits, sanitized errors and no tools/files/shell execution. Do not accept partial, refused or errored completions. Cleanup failure preserves original; translation failure follows existing recovery/no-injection semantics. Recheck authorization before cloud work, including effective per-app settings.
- Audit native dictation, remote-send, profiles, dictionary and history. Local HTTP API deliberately stays offline and must not reach subscription providers. No new semantic index is implied by this work.
- Preserve dirty tree, installed app, capture/injection/hotkey mechanics and legal gates. Changes to text processing/authorization are authorized by this request. No global account/session changes, deploy, publish or automatic payment.

## Acceptance
Unit/fixture tests cover OAuth state/nonce/audience/signature, rotation, no API fallback, complete streamed output, subprocess isolation and routing. Typecheck/build and native tests pass. UI connection states and selections verified. Live acceptance requires the person to finish official browser login; report fixtures/build separately from actual authenticated inference and installed-app acceptance.

## Sources (read 2026-10-22)
- https://developers.openai.com/siwc/token-sharing-open-source/sign-in
- https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference
- https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations
- https://code.claude.com/docs/en/legal-and-compliance
- https://code.claude.com/docs/en/cli-reference
- https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan
