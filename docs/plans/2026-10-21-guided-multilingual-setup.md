# Guided multilingual setup and installer

User requested offline and cloud options, clearer feature guidance, translation to a changeable target language, and professional onboarding/terms handling. Existing Organic visual identity remains. User explicitly approved waiting for setup completion before microphone/dictation starts.

## Scope and defaults

- Speech recognition language stays separate from output language. Output defaults to “Same as spoken”; translation is opt-in, with local Qwen3 4B or an explicitly selected configured cloud model. Never change provider automatically after failure.
- Translation failure saves the original text when disk access succeeds and prevents wrong-language insertion. Successful translated history retains source text and source language.
- Local Qwen weights are optional, downloaded explicitly with size and published SHA256 verification. Chinese output choices distinguish Simplified, Traditional and Cantonese. Existing Whisper Chinese recognition remains available; a separate Qwen3-ASR engine is not claimed to be integrated.
- Contextual help explains why, how to set up and where data goes. Help stays inside the app and links to the corresponding settings.
- Guided onboarding includes legal-document review, explicit microphone permission action, membership, shortcut and final review; failures do not silently finish setup. Generic settings saves cannot bypass setup completion.
- Configured-provider audio/text processing and Claude CLI processing have separate explicit acknowledgement switches. Provider changes invalidate the previous configured-provider choice.
- Installer uses the app icon, per-user installation, English/Portuguese/Simplified-Chinese/Traditional-Chinese selection, final Terms page and distributed notices. Draft legal text blocks new installer distribution.

## Legal/public identity boundary

The user is based in Portugal and is concerned about exposure of personal identifying information. Publisher route/public identity and contact arrangements remain undecided. Do not put the private name or contact/address/tax information from conversation into public files. Legal manifest intentionally leaves those fields unresolved. A dedicated support/privacy email is acceptable as a contact route, but does not alone resolve applicable publisher identification duties.

Draft terms/privacy reflect actual implementation. They are not final advice, a liability waiver for all situations, or a promise of GDPR/consumer/crypto compliance. Portugal's provider-identification, business-registration and token-membership implications require scoped professional review. Do not register a company, account or domain, contact anyone, or publish identity by inference.

## Ownership and verification

Backend worker: config/translation/shared model generation. UI worker: Help, language controls, onboarding/navigation. Primary: pipeline integration, recovery, legal records, cloud choices, installer checks. Verification worker: source-derived privacy/license inventory and browser fixtures. Preserve prior dirty work.

Verify native translation routing/acknowledgement/limits, same-language passthrough, setup ownership, actual offline PT→EN and Chinese sample output, history migration and recovery. Verify frontend keyboard/help/translation/onboarding/legal-draft and failure states. Verify installer config and release checker refuse unresolved drafts. Public legal approval and final installation remain separate steps.
