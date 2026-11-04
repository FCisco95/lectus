# Portfolio preparation — 4 November 2026

Status: **applied locally to an isolated master checkout; not committed or published**.

The checkout starts at public `master`, commit `d32a9ecc2b685a3cf31002f73690206e28ff5a69`. The existing development checkout was preserved. Its 202 tracked file hashes, HEAD, index hash and Git status remained unchanged. No product code, manifests, workflows, tags, releases, accounts or installed applications were changed.

## Changes

- Replaced the public README with the approved English presentation: dictation problem, access before installation, correct release downloads, backend-specific privacy, released capabilities, architecture and honest development instructions.
- Added [the media plan](media-plan.md), [claim sources](sources.md) and [release metadata](evidence/release.json).
- Copied the existing v0.7.1 logo to `media/lectus-logo.png`. The older master baseline has no `src/assets/logo.png`; documentation therefore owns its copy. No product assets were modified and no screenshot was invented.
- Recorded scoped continuation notes in [HANDOFF.md](HANDOFF.md). The historical product handoff at `docs/HANDOFF.md` remains unchanged.

## Checks completed

| Check | Result and scope |
|---|---|
| Remote/default/release | GitHub connector reconfirmed public repository, default master, and v0.7.1 with Windows x64 EXE/MSI and Apple silicon archive. Source release is only the updater bridge. |
| Approved content | Applied README matches the approved proposal except for the logo's documentation path. |
| Working tree | Of 178 tracked baseline files, only README changed. All new files are under `docs/portfolio/`. Index remains unstaged. `git diff --check` passes. |
| Links/source | The same 11 previously checked README/installer destinations returned HTTP 200 during this session; 20 claim source paths were checked against v0.7.1. No link text or remote destination changed during application. |
| Asset | Logo bytes match the reviewed source-tag asset. Browser confirmed successful image loading at the new relative path. |
| Local visual preview | Applied document inspected at 1280×900 and 390×844, light/dark. Narrow viewport width and document scroll width both measured 390 px. Earlier full-document views cover the unchanged content. |

Machine-readable preparation checks: [preparation-checks.json](evidence/preparation-checks.json). Preview screenshots and the review patch are retained in the separate proposal packet, not represented as application media. Preview used the already installed markdown renderer and an isolated agent-browser session; that session was closed. The tool reported its Chrome runtime behind stable; no installation or upgrade was performed.

## Pending

1. Authorization for commit/push or opening a PR. Preparation did not include either action.
2. Final GitHub rendering after an authorized publication step; the local HTML preview approximates Markdown layout.
3. Native demo, three screenshots, thumbnail and a verified public video destination. Confirm the installed version before recording; do not use mocked development UI as release media.

No inference, model download, native application launch, product build or product test was performed for this documentation-only change. The historical test counts in the README remain attributed to versioned documentation.
