# Setup wizard, free trial and vocabulary that sticks: design

Date: 2026-11-13. Status: awaiting approval. Size: project.

## Intent

A new user installs Lectus and, in under three minutes, sees a branded installer, learns how the product works while accepting the terms, proves the microphone works, gets a personal vocabulary, and starts a 7 day free trial. The reference for the feel is the Orca first run: each setup task sits beside a short looping demo of the feature it sets up. OpenWhispr is the reference for "it learns your words".

A second goal comes from daily use: words already in the dictionary must actually come out. Today "Mycel" is often written as "myself".

## What exists today (verified in code)

- Windows installer: `src-tauri/tauri.conf.json` sets only `installerIcon`. No header or sidebar art, so the NSIS pages look unbranded.
- First run: `src/components/Onboarding.tsx`, 7 text-only steps (welcome, privacy and terms, wallet, mic, accessibility on macOS, shortcut, review). No demo, no microphone test, no model download, no vocabulary step. It ends by telling the user to go to Models.
- Membership: `src-tauri/src/license/mod.rs`. A wallet with $20 of ORGANIC or Mycel dictates. `GRACE_DAYS = 7` only covers a wallet that was above the floor and dropped. A new user with no wallet gets zero days (`never_funded_gets_no_grace`, `Status::Unlinked`).
- Dictionary: `src-tauri/src/transcription/dictionary.rs`. Words go to Whisper as `initial_prompt` ("Vocabulary: a, b, c."). After recognition, exact find and replace runs (`apply_dictionary_spellings`, `apply_rules`), wired in `src-tauri/src/lib.rs:898` and `:951`.
- The owner's config: 68 words, 31 rules, language locked to `pt`, model Large v3 Turbo.

## Finding: why "Mycel" becomes "myself"

- The prompt is 520 characters, estimated 173 to 208 tokens. whisper.cpp keeps the last 223 tokens of the prompt (`whisper.cpp` around line 6906 and 7106 in whisper-rs-sys 0.15.0). So the list is probably not cut yet, but it is close, and "Mycel" is first, so it would be the first word lost. The exact count is not measured.
- History, 500 entries, all tagged `pt`: "Mycel" appears in 1, "myself" in 3, "my cell" style in 1. Small sample, but the hint loses most of the time.
- The only rule that leads to Mycel is `my cell`. Nothing catches `myself`.
- Conclusion: the prompt is a weak hint by design (the code comment says so), and the deterministic layer only knows exact spellings. A real English word that sounds the same passes straight through.

## Goals

1. Branded Windows installer.
2. A first run wizard that teaches while it sets up, ends with working dictation, and never dead-ends a user who has no wallet.
3. A 7 day free trial for every new install.
4. A vocabulary step that finds the user's own words in a folder they choose.
5. Dictionary words that are heard wrong get corrected, starting with a measured fix for "Mycel".

## Non-goals

- New speech engines or models (Parakeet, Qwen3-ASR), and English/Portuguese-only language detection. Next project.
- Learning automatically from later corrections. Next project. This design only creates the data it will need.
- OpenWhispr meeting notes, voice commands, chat.
- Translating the wizard. It stays in English like the rest of the app.
- macOS installer visuals and code signing.

## Anti-goals

- Anything from the folder scan leaving the computer, or file contents being stored.
- A wizard that blocks dictation during the trial because no wallet is linked.
- Trial wording that hides what happens on day 8.
- Opening the microphone before the user clicks a button that says so.

## Approaches

1. Polish only: add art and rewrite copy in the current wizard. Cheapest. Delivers no microphone test, no words, no trial.
2. Rebuild the in-app first run, brand the NSIS installer with its built-in image options, add the trial and vocabulary work. **Picked** (my call, the owner handed over the how). It reuses the settings window, the legal acceptance flow, the hotkey capture and the model downloader.
3. A custom installer that does everything before install. Rejected: an installer cannot test the microphone or download models cleanly, it is Windows only, and it replaces a template Tauri maintains.

## Design

### A. Installer branding

Add `headerImage` and `sidebarImage` (and the uninstaller header) under `bundle.windows.nsis`, generated from `src-tauri/icons/lectus-master.png`. Keep the terms page and the language selector. No custom template. The builder confirms bitmap sizes against the Tauri config schema.

### B. First run wizard

Two panes in the existing settings window. Left: the task. Right: a looping demo of what this step enables. Demos are coded animations (React and CSS using the existing tokens, light and dark, honoring reduced motion), not video files.

```
+------------------------------+----------------------------------+
| (logo) Lectus      2 of 6    |                                  |
|                              |   [ looping demo: hold key,      |
| Check your microphone        |     pill pulses, text types      |
| Device: [ Headset mic   v ]  |     into a fake chat box ]       |
| Level:  ||||||||....         |                                  |
| [ Say: "Testing Lectus" ]    |   Your voice stays on this       |
| Heard: "Testing Lectus."     |   computer.                      |
|            [Back] [Continue] |                                  |
+------------------------------+----------------------------------+
```

Steps:

1. Welcome. Logo, the core demo, and one choice: "I speak English / Portuguese / both". Sets `language` to `en`, `pt` or `auto`.
2. Privacy and terms. The existing `LegalSetup` acceptance, unchanged in mechanism. The right pane shows where audio goes: microphone, this computer, text; cloud only if enabled later. Once accepted, the recommended speech model starts downloading in the background with visible progress.
3. Microphone. Device picker, live level bar, and a test sentence that is transcribed on screen. If the model is still downloading, the level bar works and the test waits. On macOS the accessibility step follows, as today.
4. Your words. See C.
5. Shortcut. Existing capture control plus a text box to try it. The try uses the wizard's own test capture, driven by key events inside the focused wizard window. The global hotkey pipeline and its "setup not complete" gate (`src-tauri/src/lib.rs:1099`) stay untouched. The real global shortcut is proven on Home right after finishing, with a "Try it now" card.
6. Trial and membership. "7 days free, starting now. After that, hold $20 of ORGANIC or Mycel." Shows the live floor. Connect wallet now or later. Finish opens Home.

Model recommendation (my call): the wizard downloads Small for Portuguese or both, Base for English only. Reason: Base scores 13.9% word error on Portuguese against 7.7% for Small and 4.2% for Turbo in the Handy FLEURS table, and a bad first transcript is the worst first impression. There is no GPU detection in the app today (`hardware_profile` returns only CPU count and memory), and none is added. Instead the microphone test measures how long the test sentence took. If the computer is fast, step 3 offers "Use the most accurate model (Large v3 Turbo Compact, 574 MB)". The user can change it in Models at any time.

The promise on the current welcome step stays true: no microphone stream and no pre-roll buffer until the user asks for the test or finishes setup.

### C. Your words (folder scan and "say it" check)

1. The user picks a folder, for example an Obsidian vault. A Rust command walks `.md` and `.txt` files with caps on file count, total bytes and depth, skipping hidden and dependency folders.
2. It scores candidate terms: words capitalized in the middle of a sentence, mixed case (DexScreener), short all-caps (DLMM), letters with digits (Token-2022), and two or three word capitalized phrases, weighted by frequency. Words already in the dictionary are dropped.
3. The top 30 are shown as chips. The user ticks which to keep. Only ticked words are saved, into `dictionary_words`. Nothing else from the files is kept.
4. "Say it" check, optional, for up to 5 ticked words: the user says the word once. If Lectus hears something else, the wizard shows it and asks: "Heard 'myself'. Write 'Mycel' instead when I dictate in: Portuguese / English". Nothing is saved without that confirmation. Portuguese is ticked by default when the user speaks it. English is never ticked by default, with a hint to tick it only if the heard form is not a word they use. See E.

The step is skippable. If a common-word list is needed to cut noise, its license goes through `legal/third-party` and the legal check script.

### D. Free trial

- New constant `TRIAL_DAYS = 7`, separate from `GRACE_DAYS`.
- A trial start time is stored when onboarding completes. Every existing install that is not Active, linked or not, gets it on first launch of this version.
- Linking or unlinking a wallet never resets or removes the trial start. Today `unlink_wallet` deletes `license.json` and `link_wallet` writes a fresh record (`src-tauri/src/license/commands.rs:95` and `:124`), so the trial start must survive both, in that file or beside it.
- New `Status::Trial { days_left }`. It allows dictation. It applies while the window is open and the wallet is not Active, whether or not a wallet is linked.
- After day 7, status falls back to today's behavior: unlinked or locked, dictation refused, Membership explains what to do.
- Home and Membership show "Trial: N days left".
- The clock is local. Deleting `license.json` or moving the system clock restarts it. Accepted: the module already states it is a membership check, not DRM.
- `legal/TERMS.md` needs one sentence about the trial. The wording is the owner's to approve, and the legal manifest and check must be updated with it.

### E. Dictionary words that stick

Step 0, measure before fixing. Record the owner saying "Mycel" in two Portuguese sentences and two English ones (4 short WAV fixtures, kept out of git unless the owner agrees). A test harness runs each through: no prompt, current prompt, "Mycel" placed last, and a sentence-style prompt in the spoken language, and prints the exact prompt token count. The result decides the prompt change. No prompt change ships without this evidence.

Then:

- Prompt budget guard. Count tokens. If the list exceeds the budget, drop from the low-priority end, never silently from the start, and log it.
- Sound-alikes. New config list: `{ heard: "myself", word: "Mycel", languages: ["pt"] }`. Applied after recognition, whole word, case-insensitive, only when the dictation language matches. In a Portuguese sentence the English word "myself" is almost never intended, so the rule is safe there.
- Sources of sound-alikes: the wizard "say it" check, and a "Sounds like" field in the Dictionary panel. No sound-alike ships built in. `myself` to `Mycel` is an entry in the owner's own config, not a default for other users.
- When the dictation language is unknown (language set to auto and the engine reports none, as cloud speech does), language-scoped sound-alikes do not fire.
- The recordings for step 0 come from the owner. Until they exist, only the prompt change waits. The budget guard, sound-alikes and everything else proceed.
- Known limit: in an English sentence, "myself" and "Mycel" cannot be told apart by a rule. That case needs recognition confidence or a context model and belongs to the word learning project. The measurement in step 0 will show how much prompt placement helps there.

## Constraints

- AGENTS.md: ask before changing the capture, injection or shortcut pipeline. The microphone test and "say it" check need a capture path that the wizard triggers. Approval of this document is that ask. The hotkey pipeline's behavior must not change.
- No bare-modifier hotkeys. No `v*` tag and no replacing the installed app without asking.
- `beforeBundleCommand` runs `scripts/check-legal.mjs --release`; legal text changes must pass it.
- No git hooks or CI on push: run `npm run build` and `test-release.cmd` by hand.
- Legal documents are drafts. `legal/manifest.json` has `publication_approved: false` and publisher kind `undecided`. While that holds, the accept checkbox is hidden (`src/components/LegalDocuments.tsx:60`), `complete_onboarding` refuses (`src-tauri/src/legal.rs:72`), and bundling an installer exits 1. This design does not loosen that gate. Wizard steps are built and tested with mocked IPC, as the existing UI fixture scenarios do. The fresh-profile and installer acceptance items wait for the owner to approve the legal documents.
- `legal/PRIVACY.md` needs two wording changes, for the owner to approve: the microphone can open during setup when the user clicks the test, and the folder scan reads files locally and keeps only chosen words. `legal/TERMS.md` needs the trial sentence.
- Base: the current working tree on `feat/remote-dictation` (27 commits ahead of `master`, 97 changed or untracked paths, including the untracked `legal/`, `legal.rs`, `check-legal.mjs` and `LegalDocuments.tsx` this design depends on). The first build task is a checkpoint commit of that existing work on the same branch, staged by explicit path, so wizard changes stay separable. Nothing is pushed.

## Left to the builder

File and component layout, animation details, scan caps and scoring weights, the speed threshold for the "most accurate model" offer, whether the folder picker needs a new Tauri plugin, and test names.

## Suggested build order

1. E: measurement, budget guard, sound-alikes. Small, and it fixes a daily annoyance first.
2. D: trial.
3. B: wizard shell, steps 1, 2, 3, 5, 6.
4. C: folder scan and "say it" check.
5. A: installer art.

## Acceptance

- Installer pages show Lectus art (waits for legal approval, see Constraints).
- On a fresh profile, the wizard completes in under three minutes with no wallet, and dictation works right after (waits for legal approval). Until then: every step passes its mocked-IPC UI scenario.
- Unlinking and relinking a wallet does not change the trial days left.
- Home shows "Trial: 7 days left". With the clock moved 8 days ahead, dictation is refused and Membership explains why.
- A test folder with known terms produces those terms in the top 30. Nothing but ticked words is written to disk.
- The "Mycel" Portuguese fixtures come out as "Mycel".
- `npm run build` and `test-release.cmd` pass.
