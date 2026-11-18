# Dictation that follows the active computer (Lectus + Input Leap)

Planning only, 2026-11-18. Nothing was installed, built or reconfigured. Input Leap source was
read at commit `34a34fb` (last upstream commit, 2025-11-27). IL = Input Leap.

## Recommendation

1. **Prototype by coordinating the IL install that already runs on the PC.** No fork, no bundled
   component, no RAG. The change is PC-only: a fourth destination, "Follow Input Leap", that
   decides between "this computer" and "the paired Mac" for each dictation. The Mac keeps the
   shipped 0.7.1 in Receive mode, so no macOS build is needed to test the product hypothesis.
2. **Signal: the IL server's own log.** The server writes one line per screen change. The PC
   start script already logs at INFO to a file, so the signal exists today. It is not a supported
   API, but upstream is archived, so the format is frozen.
3. **Rule: the destination is the screen that was active at key-down, locked for that
   dictation.** Text is delivered at most once. On any doubt nothing is pasted remotely and the
   text stays in Dictations on the PC. If the IL server is not running, the destination is the PC.
4. **Before writing code, run Step 0** (15 minutes, today's installs). It answers the biggest
   unknown and is also the first real PC-to-Mac delivery test.
5. **Do not design the component installer yet.** Build it only if the prototype passes the gate,
   and decide then whether the engine is IL (archived) or Deskflow (active).

## Owner direction (same day, after reading the plan)

- The owner tried the shipped Send/Receive feature and did not get on with it: "not
  straightforward". So the base remote delivery counts as tried and not adopted, not as proven.
- The real goal is wider than following IL: one app that shares the **keyboard, mouse and
  microphone** between two computers on the same network, more efficient than IL, with dictation
  inside it. That is what would set Lectus apart. Coordinating IL is a stepping stone at most.
- Consequence for this plan: the prototype above still answers "is dictation that follows the
  computer worth having", but the engine question moves to the front. Three candidates:
  1. **Fork IL.** C++/Qt, GPL-2.0, archived, no upstream. Every fix is ours and stays GPL.
  2. **Own narrow engine inside Lectus**, Rust, PC to Mac only. Lectus already has a keyboard
     hook on both systems and text injection. The active computer is then known by construction,
     which removes the main unknown and the log reader. Cost: key mapping, modifier state, stuck
     keys, clipboard, elevated windows. No IL code may be copied (GPL); the design must be ours.
  3. **Deskflow as the engine.** Active, same ancestry, same licence constraints as IL.
- **Find the lag before choosing.** "More efficient" only pays off if IL itself is the cause.
  Cheap checks, in order: `ping` PC to Mac for jitter and loss (the Mac is likely on Wi-Fi, and
  IL sends mouse moves over TCP+TLS, which stalls on a lost packet); the same session on a
  release build instead of the Debug one; the same session with Deskflow. If the network is the
  cause, an own engine helps only if it moves pointer traffic to UDP. If the Debug build is the
  cause, no rewrite is needed to fix the lag.
- **Microphone: decided, dictation only.** The owner wants one microphone near his head and
  speech-to-text on both computers. So no audio streaming and no virtual audio device: the PC
  transcribes and sends the finished text. The product is "keyboard, mouse and dictation follow
  the computer you are working on", with the active computer chosen automatically.
- **First network sample** (2026-11-18, 60 pings PC to Mac, both idle): no loss, median 4 ms,
  p95 16 ms, max 108 ms, 2 of 60 over 20 ms. The typical delay is fine; the occasional spike
  is large enough to feel as a pointer stutter over TCP. One sample, not a finding. Repeat during
  a real working session and with the Mac on cable or closer to the router.

- **One-click link, never an IP.** Owner requirement: link the two computers with one click
  instead of guessing the address. In the product this is Lectus's job: pair once, then "Connect"
  finds the peer by its paired identity (the mDNS discovery that already exists for text) and
  hands the current address to the engine. Whatever engine wins must accept an address or name
  from Lectus at start, with no manual field.

## Next session: lag diagnosis and the address problem (owner authorized the installs)

Authorized 2026-11-18, for this purpose only: install a release build of Input Leap and Deskflow
on the PC and the Mac. Review each download before installing. Keep the current Debug folder and
config so the setup can be put back.

1. Address, zero install: on the Mac, check whether the PC name resolves
   (`dns-sd -G v4 <pc-name>.local`, then `ping`). If it does, point the client at the name
   instead of the IP. That alone ends the IP edits for today's setup. If it does not, note why
   (Windows firewall mDNS rule, network profile) before changing anything.
   **Done 2026-11-18.** The Mac client now uses `DESKTOP-639B5LM.local` and connects (IPv4,
   .56 to .51). Nothing changed on the PC: Wi-Fi profile is Public, but the built-in
   `mDNS (UDP-In)` rule allows Public. Windows answers per interface: on Wi-Fi with
   192.168.68.51 and a link-local IPv6 `fe80::`, on the WSL adapter with 172.27.112.1. The IL
   server listens on IPv4 only; the Mac still picked IPv4. The drop during the switch was the
   PC server being closed (`got shutdown signal` 23:54:55; the `.bat` loop ends with its
   window), not the name.
   Owner direction, same day: he would rather build the Input Leap part inside Lectus (option 2
   above).
2. Baseline with the current Debug build: 10 minutes of normal work on the Mac through the
   link, `ping` running, count the visible stutters and the disconnects in the server log.
3. Same 10 minutes on a release build of Input Leap (v3.0.2 is the last one with installers).
4. Same 10 minutes on Deskflow. Note whether it finds the other computer without an address.
5. If possible, repeat the worst case with the Mac on cable or next to the router.
6. Decide: release build or network fixes the lag, so no own engine is needed for lag; or the
   lag stays, so an own engine with pointer traffic over UDP is justified. Record the numbers
   here and the outcome in `docs/HANDOFF.md`.

## Starting point

| | In code (this branch) | Shipped (v0.7.1) | Shown on the real PC + Mac |
|---|---|---|---|
| Pairing code `LECTUS1.<id>.<key>` | `remote_dictation/transport.rs:31-68` | yes | a Mac code is saved on the PC; pairing result not recorded |
| Discovery, mDNS `_lectus-text._tcp`, IPv4, 5 s address check | `remote_dictation/mod.rs:20,144-145,201-235` | yes | no (one LAN test on a single machine) |
| Encrypted one-shot delivery, challenge per connection | `transport.rs:206-301` | yes | no |
| Never retry after submit; history saved first | `transport.rs:262-300`, `lib.rs:1053` | yes | no |
| Destination captured when the dictation starts | `lib.rs:829`, `mod.rs:247-253` | yes | no |
| Receive mode: ignores the forwarded key, mic off | `lib.rs:137`, `lib.rs:1122-1133` | yes | no |
| Mac insertion: clipboard paste with read-back | `injection/mod.rs:52-90` | yes | no |
| Follow the active computer | absent ("does not follow the mouse", handoff 2026-10-11) | no | no |

- `git diff v0.7.1 HEAD` over remote_dictation, hook, injection and the panel is 10 lines. The
  shipped build is effectively this code.
- This PC: Lectus 0.7.1 installed and running, destination "This computer", a peer code saved.
  Dictation key Right Ctrl, hold mode.
- Every handoff since 2026-10-11 says real PC-to-Mac insertion is unverified. The current branch
  is uncompiled for macOS.
- IL on this PC: server `input-leaps.exe` v3.0.3, started by a `.bat` loop with
  `--debug INFO --log %LOCALAPPDATA%\InputLeap\input-leaps.log --no-daemon`, not elevated, TLS on.
  It is a **Debug build** (ships `Qt6Cored.dll`): v3.0.3 published only Debug tarballs for
  Windows and macOS. The last installers are v3.0.2 (2024-10-12).
- IL does work between the two machines: today's log has the Mac connecting and 34 screen
  switches. It also has 40 disconnects with TLS EOF errors and bursts of 1 s reconnects.
- The server config maps `ctrl = super` for the Mac, so holding Right Ctrl on the PC holds
  Right Command on the Mac while the Mac is active.
- **Upstream is archived** (2026-07-26, [issue 2392](https://github.com/input-leap/input-leap/issues/2392)).
  Maintainers point to Deskflow (GPL-2.0, v1.27.0 on 2026-10-01) or lan-mouse (GPL-3.0, v0.11.0).

## The main unknown: which computer is active

Base URL for the links: `https://github.com/input-leap/input-leap/blob/34a34fb/`

**No supported interface exists on Windows or macOS.**
- `--screen-change-script <path>` runs a script with the new screen name, but it is compiled only
  for X11: `src/lib/inputleap/ServerApp.cpp:680-705` (`#ifdef WINAPI_XWINDOWS`), help text 150-154.
- The IPC on port 24801 (`src/lib/ipc/Ipc.h`) only relays log lines between the GUI and the
  Windows daemon. It is not a public API and the daemon is not in use here.

**What can be observed.**
- Server: `switch from "A" to "B" at x,y`, INFO, `src/lib/server/Server.cpp:412`.
- Server, when the active client drops: `jump from "A" to "<primary>"`, `Server.cpp:2167`. So a
  disconnect or Mac sleep is in the log too, and the server always falls back to the PC.
- Client: `entering screen` / `leaving screen`, `src/lib/inputleap/Screen.cpp:105,123`.
- Each line is appended and the file closed at once (`src/lib/base/log_outputters.cpp:258-272`),
  so reading the tail at key-down is current. The file rotates at a size limit (`.log.1`).

**Why key-down is the right moment.** The key press is the only instant where the user's
intent and the screen are tied together. Lectus already fixes the destination there.

| Case | What happens with the rule |
|---|---|
| Start on the Mac, cross to the PC before the text is ready | Text goes to the Mac, in the field that had focus when you started. IL leaving the Mac does not change Mac focus. One insertion. |
| Start on the PC, cross to the Mac while holding the key | Text goes to the PC. Risk: IL may swallow the key-up, see Step 0 item 4. |
| Link drops before the text is submitted | Other discovered addresses are tried, then an error. Nothing pasted. Text in Dictations. |
| Link drops after submit, before the acknowledgement | No retry, because the Mac may have pasted. Error on the PC, text in Dictations. Never a duplicate, sometimes a manual recovery. |
| Mac asleep or IL client down | Server has jumped to the PC, so dictation is local. |
| IL server not running | The keyboard cannot be on the Mac. Dictation is local. |
| Log missing or unreadable while the server runs | Follow refuses to start and says why. Destination stays "This computer". |

**Biggest unknown: does PC Lectus see the dictation key while IL relays to the Mac?** When the
Mac is active, the IL hook discards every key except the lock keys
(`src/lib/platform/MSWindowsHook.cpp:382-399,431-433`). Lectus has its own low-level hook and
always passes keys on (`src-tauri/src/hook/windows.rs:83`). Windows calls the newest hook
first. Lectus renews its hook every 60 s (`lib.rs:1590-1605`), so it is usually first, but after
an IL restart there can be up to 60 s where IL swallows the key and no dictation starts. The
shipped Send mode has the same exposure. Only the hardware can settle it.

## Step 0: smallest test (no code, 15 minutes)

1. Mac: Lectus 0.7.1, Receive, copy the code. PC: Send, Save, Test connection.
2. Cursor on the Mac, TextEdit focused. Hold Right Ctrl, say one sentence. Ten times. Count:
   started (pill on the PC), delivered once, missing, duplicated.
3. Close and reopen the IL server window, then repeat five times inside the first 60 s.
4. Hold the key on the PC, cross to the Mac, release. Does the recording stop?
5. Note whether Right Command held on the Mac causes side effects while you speak.

Result: 15/15 means build the prototype as written. Failures only in item 3 mean the hook order
is real: add a Raw Input listener for the key (not blocked by other hooks; needs the owner's
approval, it is the shortcut pipeline). Failures in item 2 mean base delivery is broken: fix that
first, which may need a Mac build.

## Architecture options

| | Coordinate the existing install | Optional component shipped by Lectus | Maintain a fork |
|---|---|---|---|
| Work | Small, PC-only | Component manager, own builds, process supervision | All of that plus a C++/Qt KVM with no upstream |
| Licence | Lectus distributes nothing | GPL-2.0 with OpenSSL exemption (`LICENSE`): ship the licence, the exact source for the binaries, no added restrictions on that part; keep it a separate process driven by CLI and stdout | Same, and every change is published |
| Signing | none | Windows: unsigned as today. Mac: Developer ID and notarization are needed in practice, because the Accessibility grant is tied to the signature | same |
| Updates | user's problem | component manifest, own cadence | same, plus security fixes are ours |
| Use | prototype | product, if the gate passes | only for a fix that cannot be had otherwise |

**To distribute, this would have to change:**
- Lectus starts the engine itself (`--no-daemon`, generated config) and reads the same switch
  lines from the child's stdout. The log-file reader becomes a pipe reader.
- Lectus feeds the engine the peer address from its own discovery, and can exchange the TLS
  fingerprints over the paired channel (`SSL/Fingerprints/TrustedServers.txt`,
  `TrustedClients.txt`, `--profile-dir`). That is what removes the script and the IP edits. The
  IL client already resolves its server name on every connect and retries every second
  (`src/lib/client/Client.cpp:127-132`, `src/lib/inputleap/ClientApp.cpp:65,210-211`).
- Own release builds for both systems, an entry in `legal/third-party/inventory.json`, a
  notice, and a source offer.
- Permissions. Windows: inbound TCP 24800 for the server; a non-elevated server cannot control
  elevated windows or UAC prompts, and the elevated service needs an admin install, which breaks
  the per-user one-click installer. Mac: Accessibility for the engine (it posts HID events,
  `src/lib/platform/OSXKeyState.cpp:569`) on top of Lectus's Accessibility and Local Network.
  `release.yml` has no Developer ID or notarization step today.
- Engine choice. IL is archived and its last Windows/macOS installers are two years old.
  Deskflow shares the same ancestry and is active. Compare them at the gate, not now.

**Local dictation stays independent.** The engine lives in its own folder and process. Lectus
never links to it. With the component missing, stopped or crashed, the destination is "This
computer", which is already the default (`remote_dictation/mod.rs:24-27`).

**Reuse test with "ShareX-style screenshot".** Reusable: a manifest (id, version, per-OS URL,
sha256, signature with the updater key, licence and source link), the resumable checked download
that `model_download.rs` already has, a versioned folder outside the app directory, start/stop
and health supervision, removal, and showing the licence. Not reusable: OS permissions (Screen
Recording is a different grant), hotkeys, the feature's UI. A screenshot feature would likely be
Rust code inside the app and need a toggle, not an installer. So build the mechanism for the
first real consumer and do not generalize before a second one exists.

## First prototype (after Step 0 passes; needs the owner's approval, it touches destination routing)

- `Follow` destination on the PC. At the existing capture point (`lib.rs:829`) it returns the
  paired Mac when the last `switch`/`jump` line targets the Mac screen and `input-leaps.exe`
  is alive, otherwise local.
- Reader: finds the log path from the running server's command line, reads new bytes at
  key-down, handles rotation, unit tests with real log lines. Kept behind one small interface so
  a stdout pipe or another engine can replace it.
- The pill shows the destination at start. Timestamps logged: key-down, record stop, transcript
  ready, send start, acknowledgement.
- Everything else unchanged: lock, at-most-once, history first. Run as a dev build next to the
  install; do not replace the installed app. Estimate: 150-250 lines of Rust, about 30 of TS.

## Hardware test

1. **Pairing**: from zero, timed, steps counted, against today's two-app setup.
2. **Crossing**: cursor both ways, 20 times.
3. **Destination**: 20 dictations alternating PC and Mac, including a crossing right before
   key-down. Count wrong computer, duplicates, missing.
4. **Mid-flight**: the first two rows of the case table, five times each.
5. **Reconnect**: Mac sleeps 2 minutes; then a new IP (Wi-Fi off/on with a new lease). Time until
   IL is back, until Lectus Test connection passes, and whether the first dictation lands. No
   manual edit allowed. Score IL and Lectus separately.
6. **Failure**: Mac Wi-Fi off right after key-up, five times; quit Mac Lectus, twice. Then
   reconnect and dictate once more. Nothing may replay.
7. **Independence**: stop the IL server, dictate on the PC.

Measured separately, each against a baseline (IL alone, and Lectus 0.7.1 in fixed Send mode):
- **Keyboard/mouse response**: 240 fps phone video, key press to character, 10 samples, in four
  states: PC with IL and Lectus stopped, IL running, IL plus Lectus, Mac through IL. Add
  `ping` PC to Mac, 100 samples. The PC lag and the Debug IL build are suspects, not findings.
- **Transcription**: record stop to transcript ready, from the Lectus log, same sentences.
- **Transcript to remote insertion**: send start to acknowledgement (`lib.rs:1082`), against
  local insertion. The acknowledgement includes a 200 ms clipboard restore on the Mac
  (`injection/mod.rs:83-86`), so the text is visible earlier than the number says.

## Advance or stop

Advance to the component phase when, over three working days and at least 40 dictations:
zero wrong-computer insertions, zero duplicates, at least 95% of Mac dictations delivered with
the link up, recovery after sleep and IP change inside 30 s with no manual edit, a median
transcript-to-Mac-insertion under 300 ms, and the owner chooses this flow over two separate apps.

Stop or rethink when: the key is not seen reliably under relay even with Raw Input; a
wrong-computer insertion cannot be explained and fixed; IL instability (40 disconnects in one day
today) dominates the experience, in which case evaluate Deskflow before any component work; or
the lag measurement points at IL itself.
