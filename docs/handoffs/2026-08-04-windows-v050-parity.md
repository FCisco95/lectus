# Lectus — HANDOFF

## Metadata

- Last Updated: 2026-08-04 (late evening, Windows PC)
- Repository: lectus
- Branch: `master` @ `e1e4c31` (pushed)
- Version: 0.4.0 in manifests (0.5.0 bump still pending — Next Actions #1)
- Next-session model: Sonnet 5 (implementation continues; no new plan needed)

## TL;DR

**Windows PC is now at v0.5.0 parity and running.** Pulled the 22-commit
macOS overhaul (`3970207..5aafa39`), resolved the one real conflict (tray
handler — upstream's unified version won; local single-instance plugin
survived and is now committed as `62b2f91`), fixed an **upstream
Windows-breaking bug** (`RunEvent::Reopen` is macOS-only and wasn't
cfg-gated — E0599 on any Windows build; fixed as `e1e4c31` and pushed, so
master now builds on both platforms). Release build green via the Vulkan
recipe, 51/51 lib tests pass, app redeployed to `C:\lt\release\chirp.exe`
and confirmed running by the user ("seems to be working").

The macOS-side handoff content below (v0.5.0 session, bugs found+fixed,
v2 visual direction, TCC gotchas) is unchanged and still authoritative for
Mac work.

## This session's commits (Windows, 2026-08-04)

1. `62b2f91` feat: single-instance plugin — second launch focuses Settings
   window (carried over from the Jul 17 tray session, was uncommitted;
   conflict resolved in favor of upstream's unified tray-click handler)
2. `e1e4c31` fix(win): cfg-gate `RunEvent::Reopen` — variant is macOS-only
   (+ regenerated Cargo.lock with tauri-plugin-single-instance)

## Windows verification status (what "working" covers)

- Build + tests + launch: ✅ verified.
- Live dictation on the new build: **not yet exercised this session** —
  user confirmed the app runs, no dictation pass done. Worth one hotkey
  test next session (watch for the onboarding first-run flag — existing
  config had `onboarding_completed` true, so onboarding was skipped).
- Items the Mac session flagged as needing Windows checking (from
  2026-08-01 handoff, still open): `tauri-plugin-autostart` toggle,
  `force_exit`/`relaunch_app` behavior, drag-region fix regression check.
- `npm audit fix` suggested during install — 2 vulns, not touched.
- Leftover: `stash@{0}` (obsolete pre-merge tray work, safe to drop);
  untracked `build-release.cmd` at repo root (helper for the vcvars
  quoting problem in Git Bash — keep or delete).

## Windows build recipe (unchanged, confirmed working today)

Full env block in `docs/handoffs/2026-07-16-phase3-measured-build.md`.
Quoting vcvars64.bat through Git Bash `cmd //c` fails — that's why
`build-release.cmd` exists; run `cmd //c build-release.cmd` or an
equivalent batch file. Kill `chirp.exe` by PID before building (running
exe locks `C:\lt\release\chirp.exe`). Frontend: `npm run build` before the
cargo build so tauri-build embeds fresh `dist/`.

---

# macOS session state (2026-08-01, authoritative for Mac)

## This session's commits (all on master, in order)

1. `a102437` feat(ui): design tokens + theme config field
2. `6ba96af` feat(settings): restyle + IA consolidation 9→6 tabs
3. `7bf2768` feat(pill): restyle per mockup — 36px idle orb, capsule+dot, shimmer bars
4. `8f07316` feat(onboarding): first-run UI — Welcome, Mic, Accessibility, Hotkey, Done
5. `8dadb29` fix(onboarding): add Continue on the Accessibility step (Relaunch-only left no forward path when already granted)
6. `ba871ca` fix(macos): avoid ggml-metal teardown crash on quit/relaunch (root cause + fix below)
7. `c5302f6` feat(settings): premium v2 direction — glass panes, no sidebar, one accent (Cisco's "more like Wispr Flow" ask, approved via Artifact mockup)
8. `348953f` fix: reopen handling + drag-region swallowing nav clicks (two bugs found live)

Plan executed: `~/.claude/plans/lectus-is-now-working-scalable-wilkinson.md`
("Track 1 commit sequence" steps 5–8) — followed through step 7 (onboarding);
step 8 (delete `docs/mockups/`) intentionally deferred, see Next Actions.

## Bugs found + fixed during live testing (read before touching related code)

- **ggml-metal crash on ANY process exit once whisper has run.**
  `warmup_engine` runs Metal init at every startup, and libc `exit()`'s
  atexit/static-destructor chain then hits a ggml-metal static
  device-registry teardown assertion (`GGML_ASSERT([rsets->data count] ==
  0)` — known upstream bug, ggml-org/llama.cpp#17869). This crashed on tray
  Quit and on the onboarding/General **Relaunch** button (both went through
  `app.exit()`/`tauri-plugin-process::relaunch()`). Fixed with
  `force_exit()` (`lib.rs`) — calls `libc::_exit()` directly, skipping the
  chain entirely. `tauri-plugin-process` is **removed** (no longer needed);
  relaunch is now our own `relaunch_app` command.
- **Tray click required two clicks** (menu, then "Settings…"). Left-click
  now opens Settings directly on both platforms; the dropdown menu
  (Settings…/Quit) moved to right-click.
- **No `RunEvent::Reopen` handler.** As an Accessory app (no Dock icon),
  clicking the app again while already running did nothing visible —
  read as "won't open" even though it was running. Now shows+focuses
  Settings on Reopen. **(Now cfg-gated to macOS — `e1e4c31`; the variant
  doesn't exist on Windows.)**
- **`data-tauri-drag-region` on the whole settings top bar** made every
  descendant (including the new segmented nav buttons)
  `-webkit-app-region: drag`, which can swallow clicks as a window-drag
  gesture on WebKit. Fixed with an explicit `no-drag` opt-out on
  interactive children.
- **Accessibility badge stuck "off" despite the OS checkbox being on** —
  not a code bug, a TCC quirk: the grant is tied to the exact binary
  signature, and every dev rebuild here is a new unsigned binary, so the
  checked box in System Settings was a stale entry for an older build.
  Fix used live: `tccutil reset Accessibility ai.organic.lectus`, relaunch,
  re-grant fresh. **This will keep recurring every rebuild until the app is
  consistently signed** — worth deciding whether that's in scope before
  the next round of live iteration, otherwise expect to re-run that reset
  each time.
- **Hotkey/dictation silently not firing after granting Accessibility** —
  expected behavior, not a bug: the low-level keyboard hook only activates
  at startup, so granting permission after the process is already running
  needs one more full quit+reopen. Confirmed working after that; full
  pipeline log: pre-roll → capture → transcribe (~107–123ms) → inject, all
  green.

## Premium v2 visual direction (mid-session change, approved)

Cisco asked for something more like Wispr Flow after seeing the first
(already-approved) mockup implemented. Answered via `AskUserQuestion`:
glass/blur surfaces + more whitespace/bigger type + no sidebar + more
neutral color (one accent, not the rainbow everywhere). Built
`docs/mockups/settings-v2.html`, published as a Claude Artifact for
sign-off, approved ("looks good"), then ported into the real app in
`c5302f6`:

- `tokens.css`: neutral ink/paper base, single refined accent blue (was
  `--lectus-blue`, now its own value), new `--accent-soft`/`-strong` and
  `--glass-blur` tokens, bumped radius scale (sm 8/md 12/lg 18) and
  `--text-xl` to 26px.
- `Settings.tsx`/`settings.css`: **left sidebar is gone.** A top bar
  (brand mark + floating glass segmented nav + version) replaces it and
  doubles as the mac drag region (see the drag-region bug above — same
  commit's fix applies here). Panes/cards/model-cards/history-items/the
  autosave toast all got real `backdrop-filter` blur.
- Parrot rainbow gradient reserved for the logo orb + the About tab's
  brand showcase; everywhere else (banner, buttons, progress bar,
  history-copy link) moved onto the neutral+accent set.

**Known gap, called out in the commit message, not yet done:** real macOS
vibrancy (`window_vibrancy::apply_vibrancy`, already plumbed in `lib.rs`)
is currently masked — `.settings-app` paints a fully opaque background, so
the glass panes are blurring a CSS gradient wash, not the real desktop.
Wiring genuine vibrancy under the glass panes (making `.settings-app`
transparent on mac specifically, letting the OS material do the blur
instead of the CSS fake) would be a nice follow-up but needs live
verification — flagged as a risk, not done blind.

## Track 1 — original scope, status

- ✅ Foundation (tokens, theme field, platform tagging)
- ✅ Settings restyle + 9→6 IA consolidation, then re-restyled again per v2
- ✅ Pill restyle (36px idle orb, capsule+dot, shimmer bars; lib.rs sizes
  64×64/240×64, both tauri.conf windows arrays kept in sync)
- ✅ Onboarding (5 steps, backend already existed from a prior session)
- ✅ Live-eyeball pass — done on Mac; Windows build+launch done 2026-08-04
- ⬜ **Version bump to 0.5.0** (`tauri.conf.json`, `src-tauri/Cargo.toml`,
  `package.json`, About footer) — **not done, do first next session**
- ⬜ Delete `docs/mockups/` (both `settings.html`/`pill.html`/`onboarding.html`/
  `tokens.mock.css` AND the new `settings-v2.html`) — not done; confirm
  Cisco doesn't want the v2 mockup kept for reference first

## Not yet re-verified after all the above changes

- Full light/dark theme toggle (Settings → General → Appearance) in the
  new v2 glass UI — toggled in earlier CSS, not re-confirmed live after
  the v2 restyle landed.
- Windows specifics from the Mac session: `tauri-plugin-autostart`
  (Launch at login toggle), removal of `tauri-plugin-process`,
  `force_exit`/`relaunch_app`, drag-region fix regression check.
  **Windows build+launch now verified (2026-08-04); runtime behavior of
  those four still unexercised.**
- Onboarding's Hotkey step ("Try it now" style live capture) — not
  exercised on either platform.
- The Models & About tab, History tab, AI Cleanup tab, Vocabulary tab —
  none specifically confirmed on either platform.
- **New (Windows):** live dictation on the rebuilt exe not yet done.

## Gotchas (carried over + new)

- `tauri.macos.conf.json` carries a **full copy of `app.windows`** —
  platform config replaces arrays wholesale, keep both files' window defs
  in sync (bit both this session for the pill size change).
- `set_icon` clears the mac template flag; `do_set_state` re-asserts
  `set_icon_as_template(true)` after every icon swap.
- Settings window is `transparent: true` on macOS only; its CSS must
  always paint an opaque content pane, UNLESS the vibrancy follow-up
  above is done, in which case this assumption changes.
- Any macOS rebuild invalidates the Accessibility TCC grant — always
  `tccutil reset Accessibility ai.organic.lectus` + relaunch after
  reinstalling a rebuilt `.app`, don't assume a stuck "off" badge is a
  code bug first.
- Installing to `/Applications/Lectus.app` for live testing:
  `npm run tauri build` → `trash /Applications/Lectus.app` (a repo hook
  blocks raw `rm -rf` on absolute paths) → `ditto <bundle> /Applications/Lectus.app`.
- **New (Windows):** `RunEvent::Reopen` and other mac-only variants must
  be `#[cfg(target_os = "macos")]`-gated — upstream master briefly broke
  the Windows build here; fixed at `e1e4c31`. Any future mac-side merge
  touching `.run()` event handling needs a Windows compile check.
- **New (Windows):** vcvars64.bat can't be quoted through Git Bash
  `cmd //c` — use a `.cmd` helper file (`build-release.cmd` at repo root).

## Build

macOS: `npm install && npm run tauri dev` (dev) or `npm run tauri build`
(release, bundles to `src-tauri/target/release/bundle/macos/Lectus.app` +
a `.dmg`). Windows recipe: `docs/handoffs/2026-07-16-phase3-measured-build.md`
(VULKAN_SDK/vcvars env block; use `build-release.cmd`).

## Next actions (priority order)

1. **Version bump to 0.5.0** across `tauri.conf.json`, `src-tauri/Cargo.toml`,
   `package.json`, About footer.
2. One live dictation pass on Windows (new build) — hotkey → speak →
   inject; confirm no regression vs the Mac-tested pipeline.
3. Delete `docs/mockups/` after Cisco confirms the v2 mockup can go.
4. Chase the still-unverified list above as they come up live.
5. Optional: `npm audit fix` (2 vulns); drop `stash@{0}`.

## Suggested skills (next session)

- `run` — relaunch the app for the Windows dictation pass
- `verify` — synthetic dictation drive if the pipeline needs re-proving headlessly
- `commit` — land the version bump + mockup deletion once confirmed
- `handoff` — re-run at session end (per project convention)
