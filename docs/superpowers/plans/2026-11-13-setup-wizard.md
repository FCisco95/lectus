# Setup Wizard, Free Trial and Vocabulary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A branded first run that teaches while it sets up, a 7 day free trial, a folder scan that finds the user's words, and dictionary words that survive being misheard.

**Architecture:** Five slices, each shippable alone, built in this order: sound-alikes and prompt budget (Rust post-processing), trial (license module plus a `trial.json` file), wizard capture backend (new Rust module that never touches the hotkey pipeline), wizard UI (new `src/components/wizard/` replacing `Onboarding.tsx`), folder scan (new Rust module plus one wizard step), installer art (config plus two bitmaps).

**Tech Stack:** Tauri 2, Rust (whisper-rs 0.16, cpal, hound, serde), React 19 + TypeScript, Vite, Playwright fixture smoke tests (`scripts/ui-smoke.mjs`).

**Spec:** `docs/superpowers/specs/2026-11-13-setup-wizard-design.md`. Read it before Task 1. Where this plan and the spec disagree, this plan wins (it is later and resolves reviewer questions).

## Global Constraints

- Branch: stay on `feat/remote-dictation`. Never push. Never create a `v*` tag. Never run `build-release.cmd` or `deploy-local.cmd` (they overwrite the installed app). Never touch `%LocalAppData%\Lectus` or `%APPDATA%\ai.organic.lectus`, except the Task 3 bias benchmark, which reads the `dictionary_words` field only and writes nothing.
- Stage files by explicit path. Never `git add -A` or `git add .`.
- Do not change the behavior of the hotkey dictation pipeline in `src-tauri/src/lib.rs` (capture loop, `setup_blocks_dictation`, `gate_blocks_dictation`, injection). The only edits allowed there are the ones named in Tasks 2, 3, 4 and 5.
- Do not edit `legal/manifest.json`. Do not weaken `legal::ensure_accepted`, `manifest_ready`, or the hidden accept checkbox in `LegalDocuments.tsx`.
- Hotkeys: never register a bare modifier.
- Native tests: `test-release.cmd` from the repo root (several minutes on first build). Frontend: `npm run build`. UI fixtures: `node scripts/ui-smoke.mjs` with the mode env var. No hook or CI runs these for you.
- The Bash tool mangles backslashes. Put Windows paths in script files or use forward slashes. One-line commit messages only with `-m`; multi-line with `git commit -F <file>`.
- New UI copy and docs: plain English, no em dashes.
- Generated output (smoke screenshots, logs, WAV recordings) is never committed.
- If a task's tests fail twice after honest attempts, stop and report to the owner. Do not try a third rewrite.

## Review Focus

1. System clock earlier than the trial start, or a corrupt `trial.json`: the app must not panic or show negative days. Clock earlier counts as day 0. Corrupt counts as absent. (Tests in Task 4.)
2. Folder scan pointed at a drive root, a folder with binary or non-UTF-8 `.txt` files, or unreadable files: stays inside its limits, reports `truncated`, skips bad files silently, never blocks the UI thread. (Tests in Task 8.)
3. Microphone missing, unplugged mid-test, or permission denied during the wizard: the error is shown, the user can continue with "Set up later", and a second `wizard_mic_start` does not open a second stream. Leaving the step or closing the window stops the capture. (Tests in Tasks 5 and 7.)
4. Model download fails or the computer is offline during the wizard: the user can still finish setup, and the microphone test explains that the model is missing rather than hanging. (Smoke scenario in Task 7.)
5. A sound-alike whose `heard` is empty, equals its `word`, or has an empty language list: never fires and never loops. (Tests in Task 2.)

---

### Task 1: Baseline and checkpoint commit

**Files:** none created. Commits existing work.

- [ ] **Step 1: Record the baseline.** Run `npm run build`, then `test-release.cmd`. Write down pass, fail and ignored counts. Expected from the last handoff: 274 pass, 0 fail, 11 ignored. If anything fails, stop and report. Do not fix unrelated failures.
- [ ] **Step 2: List what will be committed.** Run `git status --short`. Commit every tracked modification and every untracked source, doc, legal and script file. Exclude: `.context/`, `docs/reviews/`, `scripts/diagnostics/`, any `*.log`, any model or audio file.
- [ ] **Step 3: Stage by explicit path and commit.** `git commit -m "chore: checkpoint existing work before setup wizard"`.
- [ ] **Step 4: Verify.** `git status --short` shows only the excluded paths.

### Task 2: Sound-alikes (the "Mycel" fix)

**Files:**
- Modify: `src-tauri/src/config.rs` (new struct and field; add `sound_alikes` everywhere `dictionary_words` is compared, around lines 125, 151, 167)
- Modify: `src-tauri/src/transcription/dictionary.rs`
- Modify: `src-tauri/src/lib.rs:950-955`, `src-tauri/src/api_adapter.rs:69`
- Modify: `src/components/settings/types.ts`, `src/components/settings/useConfig.ts`, `src/components/settings/DictionaryPanel.tsx`

**Interfaces:**
- Produces (Rust): `config::SoundAlike { heard: String, word: String, languages: Vec<String> }` (serde default, PartialEq), `Config.sound_alikes: Vec<SoundAlike>` default empty.
- Produces (Rust): `dictionary::effective_language(configured: &str, detected: Option<&str>) -> Option<String>`. Returns the configured code unless it is `"auto"`, then the detected code, else `None`.
- Produces (Rust): `dictionary::apply_sound_alikes(text: &str, rules: &[SoundAlike], language: Option<&str>) -> String`.
- Produces (TS): `SoundAlike { heard: string; word: string; languages: string[] }`, `Config.sound_alikes: SoundAlike[]`.

- [ ] **Step 1: Write failing tests in `dictionary.rs`.**

```rust
fn alike(heard: &str, word: &str, langs: &[&str]) -> SoundAlike {
    SoundAlike { heard: heard.into(), word: word.into(), languages: langs.iter().map(|s| s.to_string()).collect() }
}
#[test] fn sound_alike_fires_only_in_its_language() {
    let r = vec![alike("myself", "Mycel", &["pt"])];
    assert_eq!(apply_sound_alikes("comprei myself ontem", &r, Some("pt")), "comprei Mycel ontem");
    assert_eq!(apply_sound_alikes("I did it myself", &r, Some("en")), "I did it myself");
    assert_eq!(apply_sound_alikes("comprei myself ontem", &r, None), "comprei myself ontem");
}
#[test] fn sound_alike_is_whole_word_and_case_insensitive() {
    let r = vec![alike("myself", "Mycel", &["pt"]), alike("my cell", "Mycel", &["pt"])];
    assert_eq!(apply_sound_alikes("Myself e my cell, myselfish", &r, Some("PT")), "Mycel e Mycel, myselfish");
}
#[test] fn degenerate_sound_alikes_never_fire() {
    let r = vec![alike("", "Mycel", &["pt"]), alike("Mycel", "Mycel", &["pt"]), alike("myself", "Mycel", &[])];
    assert_eq!(apply_sound_alikes("myself Mycel", &r, Some("pt")), "myself Mycel");
}
#[test] fn effective_language_prefers_configured_then_detected() {
    assert_eq!(effective_language("pt", Some("en")).as_deref(), Some("pt"));
    assert_eq!(effective_language("auto", Some("en")).as_deref(), Some("en"));
    assert_eq!(effective_language("auto", None), None);
}
```

- [ ] **Step 2: Run `test-release.cmd`.** Expected: compile failure (functions missing).
- [ ] **Step 3: Implement.** Reuse `replace_whole_word_ignore_case`. Language match is case-insensitive.
- [ ] **Step 4: Wire it.** In `lib.rs` and `api_adapter.rs`, run `apply_sound_alikes` first, then `apply_dictionary_spellings`, then `apply_rules`. Language argument: `effective_language(&cfg.language, detected_language.as_deref())`. In `api_adapter.rs` use the language that request ran with.
- [ ] **Step 5: Run `test-release.cmd`.** Expected: new tests pass, total rises by 4, no failures.
- [ ] **Step 6: Dictionary panel.** Add a "Sounds like" section under the replacement rules: one row per entry with fields "When Lectus hears" and "write", checkboxes "Portuguese" (`pt`) and "English" (`en`), and a remove button. Hint text: "Use this when a word is heard as a different real word. It only applies in the languages you tick."
- [ ] **Step 7: Run `npm run build`.** Expected: no type errors.
- [ ] **Step 8: Commit** the files listed above. `git commit -m "feat(dictionary): language-scoped sound-alikes"`.

### Task 3: Prompt budget guard and measurement harness

**Files:**
- Modify: `src-tauri/src/transcription/mod.rs` (`TranscribeOptions`)
- Modify: `src-tauri/src/transcription/dictionary.rs`, `src-tauri/src/transcription/local.rs`
- Modify: `src-tauri/src/lib.rs:898-899`, `src-tauri/src/api_adapter.rs:53`
- Create: `scripts/record-fixture.cmd`
- Modify: `.gitignore` (add `src-tauri/tests/audio_samples/owner/`)

**Interfaces:**
- Produces: `dictionary::PROMPT_TOKEN_BUDGET: usize = 200` (whisper.cpp keeps at most 223 prompt tokens).
- Produces: `dictionary::bias_prompt_within(words: &[String], budget: usize, count_tokens: impl Fn(&str) -> usize) -> Option<(String, usize)>`. Builds `"Vocabulary: a, b, c."` adding words in list order while the whole prompt stays within budget. Returns the prompt and how many words were left out. `None` when no word fits or the list is empty. The first words in the list are the ones kept.
- Changes: `TranscribeOptions` gains `pub vocabulary: Vec<String>`. `TranscribeOptions::from_config(language: &str, vocabulary: &[String]) -> Self` sets `vocabulary` and sets `initial_prompt` from `bias_prompt_within` with a character estimate (`|s| s.len() / 3`). `bias_prompt` stays for existing callers and tests.

- [ ] **Step 1: Failing tests** in `dictionary.rs`: with `count = |s: &str| s.len()`, `bias_prompt_within(["Mycel","Hyphae","Embalio"], 28, count)` returns `("Vocabulary: Mycel, Hyphae.", 1)`; an empty list returns `None`; a budget of 5 returns `None`; a large budget returns all words and `0`.
- [ ] **Step 2: Implement and update both callers** to pass `&cfg.dictionary_words`.
- [ ] **Step 3: Real token count for local recognition.** In `LocalWhisper::transcribe`, when `opts.vocabulary` is not empty, rebuild the prompt with `bias_prompt_within(&opts.vocabulary, PROMPT_TOKEN_BUDGET, |s| <token count from the whisper context>)` and use that instead of `opts.initial_prompt`. Confirm the tokenizer method name and signature in the whisper-rs 0.16 source under `~/.cargo/registry/src/` before writing it. If tokenizing fails, treat the count as over budget. When words were left out, `log::warn!` the number once per call.
- [ ] **Step 4: Run `test-release.cmd`.** Expected: all pass.
- [ ] **Step 5: Fixture recorder.** Add an ignored test `record_owner_fixture` in `local.rs`: reads `LECTUS_FIXTURE_NAME`, records 6 seconds with `audio::AudioCapture::start(None)`, writes 16 kHz mono 16-bit `src-tauri/tests/audio_samples/owner/<name>.wav` with `hound`. `scripts/record-fixture.cmd <name>` sets the same environment as `test-release.cmd` and runs `cargo test --release --lib record_owner_fixture -- --ignored --nocapture`.
- [ ] **Step 6: Bias benchmark.** Add an ignored test `bench_vocabulary_bias` in `local.rs`. For every WAV in the `owner` folder (skip with a printed message when empty), with `ggml-large-v3-turbo.bin` from `bench_app_data_dir()`, the language taken from the filename prefix (`pt_` or `en_`), the target word from `LECTUS_BENCH_TARGET` (default `Mycel`), and the word list read from the `dictionary_words` array of `bench_app_data_dir()/config.json` (read that one field only), print the transcript and prompt token count for five variants: no prompt; current prompt; target word moved last; a sentence in the spoken language ("Neste texto falamos de a, b, c." or "This text mentions a, b, c."); target word alone.
- [ ] **Step 7: Commit.** `git commit -m "feat(dictionary): prompt token budget and bias benchmark"`.
- [ ] **Step 8: Do not change the prompt wording.** That waits for the owner's recordings. Add to your final report: "Run `scripts\record-fixture.cmd pt_mycel_1`, `pt_mycel_2`, `en_mycel_1`, `en_mycel_2`, saying a sentence with Mycel each time, then ask me to run the bias benchmark."

### Task 4: Free trial

**Files:**
- Modify: `src-tauri/src/license/mod.rs`, `src-tauri/src/license/commands.rs`
- Modify: `src-tauri/src/lib.rs` (`complete_onboarding` at 294, state setup at 1232)
- Modify: `src/components/settings/types.ts`, `src/components/settings/HomePanel.tsx`, `src/components/settings/MembershipPanel.tsx`
- Modify: `legal/TERMS.md` (one draft sentence, see Step 8)

**Interfaces:**
- Produces: `license::TRIAL_DAYS: i64 = 7`.
- Produces: `Status::Trial { days_left: i64, pubkey: Option<String> }` (serializes as `kind: "trial"`). `allows_dictation` returns true for it.
- Changes: `status_of(license: Option<&License>, trial_started_ms: Option<i64>, now_ms: i64) -> Status`. Compute today's verdict first. If it allows dictation, return it. Otherwise, if a trial start exists and `now_ms.saturating_sub(start) < TRIAL_DAYS * MS_PER_DAY`, return `Trial` with days rounded up the same way `Grace` rounds, and `pubkey` set when a wallet is linked. Otherwise return today's verdict.
- Produces: `trial_file(dir: &Path) -> PathBuf` (`trial.json`), `load_trial(path: &Path) -> Option<i64>` (corrupt or non-positive is `None`), `start_trial_if_absent(path: &Path, now_ms: i64) -> anyhow::Result<i64>` (atomic write like `save_to`; returns the existing start when valid, never overwrites it).
- Changes: `LicenseState::new(license: Option<License>, trial_started_ms: Option<i64>)`, new field `trial_started_ms: Mutex<Option<i64>>`, `status()` passes it through.
- Produces (TS): `LicenseStatus` gains `{ kind: 'trial'; days_left: number; pubkey?: string | null }`. `allowsDictation` returns true for it.

- [ ] **Step 1: Failing tests in `license/mod.rs`.** Update existing `status_of` calls to pass `None`. Add: unlinked with a trial started now is `Trial { days_left: 7, pubkey: None }`; started 6.5 days ago gives `days_left: 1`; started exactly 7 days ago is `Unlinked`; linked wallet never funded with a trial started 1 day ago is `Trial { days_left: 6, pubkey: Some(..) }`; an `Active` wallet with a trial stays `Active`; a `Grace` wallet with a trial stays `Grace`; `now` earlier than the start gives `days_left: 7`; `start_trial_if_absent` called twice in a tempdir returns the first value both times; a `trial.json` containing `garbage` loads as `None`.
- [ ] **Step 2: Run `test-release.cmd`.** Expected: compile failure.
- [ ] **Step 3: Implement** in `license/mod.rs`.
- [ ] **Step 4: Commands.** Every `status_of` call in `commands.rs` goes through `license_state.status(now)`. `unlink_wallet` still deletes only `license.json` and now returns and announces `license_state.status(now)` instead of a hard-coded `Unlinked`. `link_wallet` is otherwise unchanged. Neither touches `trial.json`.
- [ ] **Step 5: Start points in `lib.rs`.** At startup, load `trial.json` into `LicenseState`; if `onboarding_completed` is already true and no trial start exists, call `start_trial_if_absent`. In `complete_onboarding`, after the config is saved, call `start_trial_if_absent`, store the value in `LicenseState`, and emit `license-changed` with the new status. A failed write is logged and must not fail onboarding.
- [ ] **Step 6: Run `test-release.cmd`.** Expected: all pass.
- [ ] **Step 7: Frontend.** Home status line for `trial`: "Free trial: N days left. After that, hold $20 of ORGANIC or Mycel." (use the live floor when loaded, singular "1 day left"). Membership panel: the same line as a banner above the connect button; when `pubkey` is set, also show the linked wallet. Fix every place TypeScript now flags as non-exhaustive. Run `npm run build`.
- [ ] **Step 8: Draft legal sentence.** Add to `legal/TERMS.md`, in the section that describes membership: "New installations can use dictation for 7 days without a linked wallet. After that period, dictation requires a linked wallet that meets the membership requirement described above." Keep every draft marker. Run `node --test scripts/check-legal.test.mjs`. Expected: pass.
- [ ] **Step 9: Commit.** `git commit -m "feat(license): 7 day free trial"`.

### Task 5: Wizard capture backend

**Files:**
- Create: `src-tauri/src/setup_wizard.rs`
- Modify: `src-tauri/src/lib.rs` (`mod`, `.manage`, `generate_handler!` at 1248)

**Interfaces (Tauri commands, all new):**
- `wizard_mic_start(device: Option<String>) -> Result<(), String>`. Starts a dedicated thread that owns an `audio::AudioCapture` (the stream is not `Send`), drains it every 50 ms into a shared buffer capped at 15 seconds (240,000 samples, oldest dropped), and emits `wizard-audio-level` (`f32`, 0 to 1) each tick. A second call while running returns `Ok(())` without opening a second stream. Refuses with "A dictation is in progress." when `Activation.recording_active` is set.
- `wizard_mic_stop() -> ()`. Stops the thread, clears the buffer, emits `wizard-audio-level` 0.
- `wizard_transcribe(raw: bool) -> Result<WizardTranscript, String>`. Takes and clears the buffer. Errors: "Nothing was recorded." for under 0.3 seconds; "The speech model is not ready yet." when the local engine is not loaded. Runs through the existing `worker::TranscribeWorker` so it never overlaps a dictation, with `TranscribeOptions::from_config` and the VAD path exactly as the pipeline builds them. `raw: true` returns the recognizer's text untouched. `raw: false` applies sound-alikes, spellings and rules.
- `WizardTranscript { text: String, language: Option<String>, audio_ms: u64, elapsed_ms: u64 }`.
- Pure helpers for tests: `push_capped(buf: &mut Vec<f32>, chunk: &[f32], cap: usize)`, `level_from_rms(rms: f32) -> f32` (`(rms * 6.0).clamp(0.0, 1.0)`).

Audio stays in memory only. Nothing here writes a file, calls injection, or touches the hotkey hook.

- [ ] **Step 1: Failing tests** in `setup_wizard.rs`: `push_capped` keeps the newest `cap` samples; `level_from_rms(0.0) == 0.0`, `level_from_rms(1.0) == 1.0`, `level_from_rms(0.05)` is about `0.3`.
- [ ] **Step 2: Implement** the helpers, the managed state (`Arc<Mutex<Vec<f32>>>` plus an `AtomicBool`), and the three commands. Register them.
- [ ] **Step 3: Run `test-release.cmd`.** Expected: all pass.
- [ ] **Step 4: Commit.** `git commit -m "feat(wizard): test capture commands"`.

### Task 6: Wizard shell, welcome, terms, trial and finish

Load the `frontend-design:frontend-design` skill before writing UI. Use the existing tokens in `src/styles/tokens.css` and `BrandMark`. Read `scripts/ui-smoke.mjs` to learn how fixtures mock IPC before adding scenarios.

**Files:**
- Create: `src/components/wizard/SetupWizard.tsx`, `src/components/wizard/useWizardState.ts`, `src/components/wizard/models.ts`
- Create: `src/components/wizard/steps/WelcomeStep.tsx`, `TermsStep.tsx`, `AccessibilityStep.tsx`, `TrialStep.tsx`
- Create: `src/components/wizard/demos/DictationDemo.tsx`, `PrivacyFlowDemo.tsx`, `TrialDemo.tsx`
- Create: `src/styles/wizard.css`
- Modify: `src/components/Onboarding.tsx` (becomes `export { SetupWizard as Onboarding } from './wizard/SetupWizard';`), `scripts/ui-smoke.mjs`
- Delete: `src/styles/onboarding.css` once nothing imports it

**Interfaces:**
- `SetupWizard({ onComplete }: { onComplete: () => void })`. Same contract as today's `Onboarding`.
- Step ids in order: `welcome`, `terms`, `mic`, `accessibility` (macOS only), `words`, `shortcut`, `trial`. Tasks 7 and 9 add `mic`, `shortcut` and `words`; until then those ids render a placeholder with Back and Continue.
- Each step component receives `{ state: WizardState; next: () => void; back: () => void }` and renders its own left pane. `SetupWizard` renders the header (logo, "N of M"), the progress bar, and the right pane demo for the current step.
- `models.ts`: `recommendedModel(choice: 'en' | 'pt' | 'both'): string` returns `ggml-base.bin` for `en`, `ggml-small.bin` otherwise. `ACCURATE_MODEL = 'ggml-large-v3-turbo-q5_0.bin'`. `isFast(modelName: string, audioMs: number, elapsedMs: number): boolean` is true when `audioMs / elapsedMs` is at least 15 for Small and at least 30 for Base.
- `useWizardState` owns: the config snapshot, license status, floor, legal readiness, the language choice, and model download status (from `get_models_status` and the `model-download-state` event).

Behavior:
- Layout: two panes, task on the left, demo on the right. Under 820 px wide the demo pane moves below the task. Demos are React plus CSS animations, work in light and dark, and stop moving under `prefers-reduced-motion`.
- Welcome: logo, heading "Talk. Lectus types.", `DictationDemo`, and one choice "I speak": English, Portuguese, Both. Continue saves `language` (`en`, `pt`, `auto`) with the fresh-snapshot pattern the current `finish()` uses (`get_config`, change one field, `save_config`).
- Terms: renders the existing `LegalSetup`. Continue is disabled until it reports ready. `PrivacyFlowDemo` shows microphone, this computer, text, with a dashed "cloud, only if you turn it on" branch. When acceptance succeeds, start `download_model` for `recommendedModel(choice)` if it is not downloaded, then `select_model`. Download errors are shown and never block Continue.
- Accessibility (macOS): move the current step's content unchanged.
- Trial: heading "7 days free, starting now". Body: "After 7 days, dictation needs a wallet holding $20 of ORGANIC or Mycel. Linking asks for a signed message. It never moves SOL or tokens." (live floor when loaded). Buttons: "Connect wallet" (existing `link_wallet` flow and its waiting and error text) and the primary "Start my free trial". When the wallet is already eligible the primary reads "Finish setup". The primary runs today's `finish()` logic: save the chosen hotkey with a fresh snapshot, `complete_onboarding`, `onComplete()`.
- After finishing, Home shows a one-time card: "Try it now: click into any text field, hold {shortcut} and speak." with a Dismiss button. Track it with a `localStorage` flag set by the wizard on finish and cleared on dismiss or after the first successful dictation event. This is where the real global shortcut is first used.

- [ ] **Step 1: Add smoke mode `UI_SMOKE_WIZARD=1`** to `scripts/ui-smoke.mjs`, default output `docs/reviews/2026-11-13-wizard-smoke`. Scenarios for this task: the welcome step saves `auto` when "Both" is chosen; Continue on the terms step stays disabled when legal status is not ready; with ready and accepted mocked, `download_model` is invoked with `ggml-small.bin` for "Both" and `ggml-base.bin` for "English"; the trial step shows the floor and "Start my free trial" calls `complete_onboarding`; light and dark screenshots of each step at 960x680 and 720x540.
- [ ] **Step 2: Run** `UI_SMOKE_WIZARD=1 node scripts/ui-smoke.mjs`. Expected: the new scenarios fail.
- [ ] **Step 3: Build the shell, steps and demos.**
- [ ] **Step 4: Run** `npm run build`, then the wizard smoke, then `UI_SMOKE_GUIDED=1 node scripts/ui-smoke.mjs` with `UI_SMOKE_OUTPUT` set to a new dated folder. Update guided scenarios that asserted old onboarding text. Expected: all pass.
- [ ] **Step 5: Look at the screenshots yourself.** Fix clipped text, overlap, and anything that looks unstyled.
- [ ] **Step 6: Commit** source and script changes only. `git commit -m "feat(wizard): two-pane setup shell with welcome, terms and trial"`.

### Task 7: Microphone and shortcut steps

**Files:**
- Create: `src/components/wizard/steps/MicStep.tsx`, `ShortcutStep.tsx`, `src/components/wizard/demos/MicDemo.tsx`, `ShortcutDemo.tsx`, `src/components/wizard/useWizardMic.ts`
- Modify: `src/components/wizard/SetupWizard.tsx`, `scripts/ui-smoke.mjs`

**Interfaces:**
- Consumes: `wizard_mic_start`, `wizard_mic_stop`, `wizard_transcribe`, event `wizard-audio-level`, `WizardTranscript` (Task 5); `isFast`, `ACCURATE_MODEL` (Task 6).
- Produces: `useWizardMic(): { level: number; recording: boolean; error: string; start(device?: string): Promise<void>; finish(raw: boolean): Promise<WizardTranscript | null>; cancel(): Promise<void> }`. It calls `wizard_mic_stop` on unmount.

Behavior:
- Mic step: device picker from `list_input_devices` (first option "System default"); choosing one saves `input_device` with the fresh-snapshot pattern. "Test my microphone" starts the capture and shows the level bar. "Show what I said" calls `finish(false)` and prints the text under "Heard:". While the model is downloading the button is disabled with the progress shown; if the download failed it reads "The speech model is not downloaded yet. You can test later in Home." Keep the existing permission badge, the "Request microphone access" button and the "Set up later" path. If `isFast` is true for the test, show "This computer is fast. Use the most accurate model (574 MB)?" with one button that downloads and selects `ACCURATE_MODEL` in the background.
- Shortcut step: the existing `HotkeyCapture`, then a "Try it" text box. While the wizard window is focused, pressing the chosen key starts `useWizardMic` and releasing it calls `finish(false)` and appends the text (toggle mode: press starts, press again finishes). Map `KeyboardEvent.code` with the existing table in `types.ts`. This never registers or triggers the global shortcut. Hint under the box: "After setup this shortcut works in every app."

- [ ] **Step 1: Smoke scenarios** (fail first): the level bar follows mocked `wizard-audio-level` events; "Show what I said" prints the mocked text; a mocked `wizard_mic_start` rejection shows the error and "Set up later" still advances; a fast mocked transcript shows the accurate model offer and a slow one does not; a failed model download shows the missing-model message and does not block Continue; leaving the mic step invokes `wizard_mic_stop`; in the shortcut step a key down and key up on the chosen key appends the mocked text.
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run** `npm run build` and the wizard smoke. Expected: all pass. Review the screenshots.
- [ ] **Step 4: Commit.** `git commit -m "feat(wizard): microphone test and shortcut try"`.

### Task 8: Folder scan backend

**Files:**
- Create: `src-tauri/src/vocabulary_scan.rs`
- Modify: `src-tauri/src/lib.rs` (`mod`, `generate_handler!`), `src-tauri/Cargo.toml`, `package.json`, the capabilities file under `src-tauri/capabilities/`

**Interfaces:**
- `TermCandidate { term: String, count: u32, score: f32 }`, `ScanReport { candidates: Vec<TermCandidate>, files_read: u32, truncated: bool }` (both `Serialize`).
- `ScanLimits { max_files: 5000, max_total_bytes: 50 MB, max_file_bytes: 1 MB, max_depth: 12 }` with `Default`.
- `extract_candidates(texts: &[String], existing: &[String], top: usize) -> Vec<TermCandidate>` (pure).
- `scan_folder(root: &Path, limits: &ScanLimits, existing: &[String]) -> ScanReport`.
- Command `scan_folder_for_terms(path: String) -> Result<ScanReport, String>`: runs `scan_folder` in `spawn_blocking` with the current `dictionary_words` and `top = 30`. Errors only when the path is not a readable directory.
- Folder picker: add the official Tauri dialog plugin (`tauri-plugin-dialog` and `@tauri-apps/plugin-dialog`) with only the open-dialog permission. Check current setup steps with Context7 before adding it.

Rules for `extract_candidates`:
- Before tokenizing, drop YAML front matter, fenced code blocks, inline code, URLs and Markdown link targets. Strip `[[` and `]]`.
- Candidate classes and weights: mixed case inside a word such as `DexScreener` (3); all capitals of 2 to 6 letters such as `DLMM` (2); letters with digits, including a hyphen, such as `Token-2022` (2); a capitalized word that follows a lowercase word in the same sentence (1); a run of 2 or 3 such capitalized words, as one term (1.5).
- Drop a term when: it occurs fewer than 2 times; it matches an existing dictionary word ignoring case; its lowercase form occurs mid-sentence more often than its capitalized form; it is in the stop list (English and Portuguese month and weekday names, `I`, `OK`, `TODO`, `FIXME`, `README`).
- Score is weight times `ln(1 + count)`. Sort by score, then by term. Return the top `top`.

Rules for `scan_folder`: only `.md`, `.markdown`, `.txt`; skip directories whose name starts with `.` and `node_modules`, `target`, `dist`, `build`; do not follow symlinks; skip files over `max_file_bytes` or not valid UTF-8; stop and set `truncated` when a limit is reached. File contents are never stored or logged.

- [ ] **Step 1: Failing tests.** `extract_candidates` on fixture strings finds `DexScreener`, `DLMM`, `Token-2022` and `Magic Eden`; ignores `The` at sentence starts, words inside a code fence, a URL, a word seen once, and `mycel` when `Mycel` is in `existing`; orders a term seen 10 times above the same class seen twice. `scan_folder` in a tempdir: reads `a.md` and `b.txt`, ignores `c.rs` and `.hidden/d.md`, skips a `.txt` containing bytes `0xFF 0xFE` without error, and returns `truncated: true` with `max_files: 1`.
- [ ] **Step 2: Run `test-release.cmd`.** Expected: compile failure.
- [ ] **Step 3: Implement.** Register the command and the dialog plugin.
- [ ] **Step 4: Run `test-release.cmd` and `npm run build`.** Expected: all pass.
- [ ] **Step 5: Commit.** `git commit -m "feat(vocabulary): local folder scan for candidate terms"`.

### Task 9: "Your words" step

**Files:**
- Create: `src/components/wizard/steps/WordsStep.tsx`, `src/components/wizard/demos/WordsDemo.tsx`
- Modify: `src/components/wizard/SetupWizard.tsx`, `scripts/ui-smoke.mjs`, `legal/PRIVACY.md`

**Interfaces:** Consumes `scan_folder_for_terms` and `ScanReport` (Task 8), `useWizardMic` (Task 7), `SoundAlike` (Task 2).

Behavior:
- "Choose a folder" opens the folder picker, then scans. Privacy line under the button: "Lectus reads these files on this computer only. It keeps the words you tick and nothing else."
- Results: up to 30 chips, all unticked, with "N files read" and, when truncated, "Large folder: showing the best matches from the first files." No results: "No unusual words found. You can add words later in Dictionary."
- "Add N words" merges the ticked terms into `dictionary_words` (no duplicates ignoring case) using the fresh-snapshot pattern.
- "Say it" check, optional, for the first 5 added words. One word at a time: "Say: Mycel", hold the button to record, release to call `finish(true)`. Compare after trimming punctuation and ignoring case. Equal: "Heard correctly." Different and 1 to 4 words long: show "Heard 'myself'. Write 'Mycel' instead when I dictate in:" with checkboxes Portuguese and English. Portuguese is ticked when the welcome choice was Portuguese or Both. English is never ticked by default and carries the hint "Tick only if this is not a word you use." "Save" appends `{ heard, word, languages }` to `sound_alikes`; it is disabled when no language is ticked. Anything else: "Could not hear a single word. Try again." and nothing is saved.
- "Skip" is always available.

- [ ] **Step 1: Smoke scenarios** (fail first): only ticked chips reach `save_config`; an existing word is not duplicated; a mocked heard form of `myself` for `Mycel` with choice "Both" shows Portuguese ticked and English unticked and saves `{ heard: "myself", word: "Mycel", languages: ["pt"] }`; an identical heard form saves nothing; a six-word heard form saves nothing; Skip advances with no `save_config` call.
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Draft privacy sentences** in `legal/PRIVACY.md`, keeping every draft marker. In the microphone section: "During first-run setup the microphone opens only when you start the microphone test or a word check, and that audio is kept in memory and discarded after it is transcribed." New short section "Vocabulary folder scan": "If you choose a folder during setup, Lectus reads its text files on your device to suggest vocabulary. Only the words you select are saved. File contents are not stored and do not leave your device." Run `node --test scripts/check-legal.test.mjs`.
- [ ] **Step 4: Run** `npm run build` and the wizard smoke. Review screenshots.
- [ ] **Step 5: Commit.** `git commit -m "feat(wizard): find your words from a folder, with sound-alike check"`.

### Task 10: Installer art

**Files:**
- Create: `scripts/gen_installer_art.ps1`, `src-tauri/icons/installer-header.bmp`, `src-tauri/icons/installer-sidebar.bmp`
- Modify: `src-tauri/tauri.conf.json` (`bundle.windows.nsis`)

- [ ] **Step 1: Confirm key names and bitmap sizes** in `node_modules/@tauri-apps/cli/config.schema.json` (search `headerImage`, `sidebarImage`, `uninstallerHeaderImage`). Expected: header 150x57, sidebar 164x314. Use what the schema says.
- [ ] **Step 2: Write the generator**, in the style of `scripts/gen_tray_icons.ps1`, from `src-tauri/icons/lectus-master.png`: header is the mark at the right on the app's dark surface color; sidebar is the mark centered in the upper third with the wordmark "Lectus" under it. Output 24-bit BMP.
- [ ] **Step 3: Run it and verify** both files exist with the exact pixel sizes. Open both images and check they are not stretched or clipped.
- [ ] **Step 4: Add the keys** to `tauri.conf.json`. Add the uninstaller header only if the schema has it. A full bundle cannot run until the legal documents are approved, so verification stops at: valid JSON, key names present in the schema, `npm run build` passes.
- [ ] **Step 5: Commit.** `git commit -m "feat(installer): branded NSIS header and sidebar"`.

### Task 11: Close out

- [ ] **Step 1: Full verification.** `npm run build`, `test-release.cmd`, wizard smoke, guided smoke. Record the counts next to the Task 1 baseline. Use the superpowers:verification-before-completion skill before claiming anything passes.
- [ ] **Step 2: Handoff.** Archive the current `docs/HANDOFF.md` to `docs/handoffs/archive/HANDOFF-2026-11-13.md`, then rewrite `docs/HANDOFF.md` (about 80 lines): what shipped, the test counts, and the open items: owner approves the legal documents (blocks fresh-install and installer acceptance); owner approves the three draft legal sentences; owner records the four fixtures, then the bias benchmark decides the prompt wording; English-sentence "myself" confusion belongs to the word-learning project; next projects are English/Portuguese-only detection, new speech engines, and learning from corrections. Keep the still-pending subscription AI login test from the old handoff.
- [ ] **Step 3: Delete** this plan and the spec once the handoff carries the open items. Remove generated smoke folders.
- [ ] **Step 4: Commit.** `git commit -m "docs: handoff after setup wizard"`. Do not push.
- [ ] **Step 5: Report to the owner** in a few lines: what works, test counts, and the single next thing you need from them.
