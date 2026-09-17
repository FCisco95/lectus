# Agent brief — finish the Lectus app shell (unattended)

**You own this to completion. Do not ask the user questions.** When a choice
appears, take the recommended option below (or the smallest reversible one),
write the decision into `docs/HANDOFF.md`, and keep going. The user is away.

## Mission

Finish the approved app-shell redesign on branch `feat/app-shell-canvas`, leave
every check green, and update the handoff. The design is already decided and
signed off — implement it, do not re-open it.

**Spec:** `docs/superpowers/specs/2026-09-17-app-shell-canvas-design.md` — read it
first, in full. It is the contract.

**Repo:** `C:\Users\joao_\Desktop\DEVELOPMENTS\lectus` (Tauri 2 · Rust ·
React + TS). Branch `feat/app-shell-canvas`, last commit `9514bb3`, nothing
pushed. `master` is at `dfff2cb`.

Also read `CLAUDE.md` and `docs/HANDOFF.md` before touching anything.

## State right now

`9514bb3` is a deliberate WIP commit. **It does not compile.** Already done:

- `src-tauri/tauri.conf.json` — settings window is `decorations: false`, 960×680
- `src/components/Titlebar.tsx` — drag region, Segoe Fluent glyph buttons, close
  calls `close()` so the existing `CloseRequested` handler hides the window
- `src/styles/settings.css` — appended `.app-frame` / `.titlebar` block at the end
- `src/App.tsx` — renders `<Titlebar/>` above every non-pill screen
- `src/components/settings/useConfig.ts` — config load + debounced save, lifted
  out of `Settings.tsx` with behaviour unchanged
- `src/components/Sidebar.tsx`, `SettingsModal.tsx`, `Shell.tsx` — scaffolded

The release binary at `C:\lt\release\chirp.exe` was built from this commit and is
**running now** (it only contains the titlebar work, not the shell).

## Tasks, in order

Commit after each numbered item, with the verification gate green. Conventional
commit messages, and end each with:
`Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`

1. **Make it compile.**
   - Create `src/components/settings/DictationsPanel.tsx`: the history list,
     filter, copy and "Clear history" currently living in `HomePanel.tsx`. Props:
     `{ config: Config }`. It owns the only scroll in the app.
   - Rewrite `HomePanel.tsx` to the spec's Home: membership banner → greeting →
     three stat cards → status line → empty-state hint. **No list.**
   - Point `App.tsx` at `Shell` instead of `Settings`, then delete
     `src/components/Settings.tsx` once nothing imports it.
   - `Icon.tsx` needs a `help` glyph if you add the Help row; skip Help entirely
     if it costs more than a link to the GitHub README.

2. **Canvas CSS.** Sidebar sits bare on `--bg-window`; the content surface is one
   card with a margin (`--bg-card`, `--radius-lg`, 1px `--border`). Window
   buttons stay top-right over the card. Style `.sidebar`, `.sidebar-item`,
   `.sidebar-foot`, `.surface`, `.modal-backdrop`, `.modal-panel`, `.modal-rail`,
   `.modal-body`, `.modal-close` — the components already reference these names.
   Scrollbars: thin, overlay, transparent track, fading thumb, extending the
   pattern at `settings.css:145`. Do a deliberate dark-theme pass; the tokens
   exist in `tokens.css`, so use them and never hardcode a colour.

3. **The greeting name.** Add `display_name: String` to `config.rs` (empty =
   derive) with the same `#[serde(default)]` discipline as `onboarding_completed`.
   Add a Tauri command returning the OS display name: try `GetUserNameExW`
   (`NameDisplay`, secur32, declared by hand the way `libc` already is in this
   repo), fall back to `%USERNAME%` prettified. Surface the field at the top of
   `GeneralPanel`. Unit-test the prettifier (`joao_` → `Joao`).

4. **Streak.** Add `streak_days` to the stats the Home cards read. Derive it from
   existing `history.json` timestamps in `history.rs` — no new capture fields, so
   it works retroactively. Unit-test it: empty history, single day, a gap that
   breaks the streak, and the today-vs-yesterday boundary.

5. **Resize handles — do this unconditionally.** The spec called for a manual
   spike (hover Snap Layouts, drag the edges) that only a human at the keyboard
   can run, so do not try to verify it. Implement six invisible 6px edge strips
   calling `startResizeDragging()`, guarded so they are inert on macOS. If the
   native borders turn out to work, the strips are harmless. Note in `HANDOFF.md`
   that Snap Layouts on the maximise button is **unverified and needs a human**.

6. **Full pass.** Walk every surface and the modal in both themes; fix spacing,
   focus rings, keyboard tab order, and the Esc-closes-modal path.

## Verification gate — run all three before every commit

```
npm run build                                   # tsc + vite, must be clean
powershell -Command "& cmd.exe /c 'C:\Users\joao_\Desktop\DEVELOPMENTS\lectus\build-release.cmd'"
cd src-tauri && cargo test --release --lib      # 115 tests today; must not drop
```

`cargo test` needs the same env as the build (Ninja + vcvars + `CARGO_TARGET_DIR=C:\lt`).
**A plain `cargo check` fails** in the debug cmake path — never use it as a signal.

Windows gotchas that have cost time before:

- Call `cmd.exe` through PowerShell. From Git Bash, `cmd /c foo.cmd` gets its
  `/c` mangled into a path and silently does nothing, returning exit 0.
- Multi-line commit messages: heredoc only.
- Kill a process by PID from the port or process list, never `taskkill /IM`.

To see your work: stop the running instance and start the fresh build.

```
powershell -Command "Get-Process chirp -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep -Milliseconds 800; Start-Process 'C:\lt\release\chirp.exe'"
```

The window starts hidden — the tray icon or the pill opens it. Screenshots need a
human or Computer Use; `tauri-plugin-mcp` is still unwired, so do not burn time
trying to screenshot the webview.

## Do not touch

- Audio capture, text injection, the hotkey pipeline, the transcription worker.
- The Organic token gate (`src-tauri/src/license/`) and `gate_blocks_dictation`.
- The pill window and `pill.css`.
- `docs/superpowers/specs/2026-09-17-local-transcription-api.md` — a separate,
  unstarted slice. Do not implement the HTTP API.
- Do not `git push`, do not open a PR, do not touch `master`.
- No API keys or RPC keys in code — the repo is public.

## When the work is done

1. Refresh `docs/HANDOFF.md` and drop a dated snapshot in `docs/handoffs/`.
   Include: what shipped, decisions you made alone, the unverified Snap Layouts
   item, and a "suggested skills" section for the next session.
2. Leave the branch committed and unpushed, working tree clean.
3. Final message: what now works, how to see it, what you chose without asking,
   and anything you could not finish and why.

## If you get stuck

Do not stop the whole run. Park the blocked item with a written note in
`HANDOFF.md`, finish everything else, and say plainly at the end what is
incomplete. A partially delivered task that is honestly reported beats a stalled
session waiting on a question nobody is there to answer.

## Only if 1–6 are done and green

In priority order, each its own commit, each stopping at a green gate:

1. What's-new modal shown once after an update (release notes from the updater).
2. `Insights` surface from the spec's "not in this spec" list — WPM and per-app
   usage need two new fields on `HistoryEntry` (speech duration, focused app at
   trigger time), recorded going forward; old entries cannot be backfilled, so
   the graphs start empty and must say so.
