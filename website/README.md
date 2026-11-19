# Lectus website

Static marketing site: plain HTML, CSS and JavaScript modules. No build step, no dependencies,
no cookies, no third-party requests (fonts are self-hosted).

## Preview

```sh
cd website
python -m http.server 4173 --bind 127.0.0.1
```

Open <http://127.0.0.1:4173/>. Add `?placeholders` to any page to outline everything that still
waits for real content.

## Pages

| Path | What it is |
|---|---|
| `index.html` | Home: hero demo, speed race, hold-the-key tryout, features, privacy, remote dictation, local API, install, pricing, questions |
| `download/` | Download button, requirements, three install steps, first-run walkthrough, fixes, updates, uninstall, admin switches, Mac status |
| `changelog/` | What's new, one entry per release, written by hand |
| `privacy/`, `terms/` | Plain summaries. The final legal texts are placeholders |
| `404.html` | Not-found page. Uses root-absolute paths, so it needs the site at the domain root |

`assets/css`: `base.css` (tokens, type, header, footer, buttons), `demo.css` (the pill, desktop
stage, app windows), `home.css` (home sections), `pages.css` (other pages).
`assets/js`: `site.js` (menu, OS detection, release data, motion switch), `demos.js` (all demos),
`download.js` (download start and first-run walkthrough).

The header and footer are repeated in every HTML file. Change them in all six.

## Download flow

Every download button goes to `download/?dl=1`. On a Windows PC that page starts the download
by itself and shows "Your download has started" with the three steps. On a Mac, Linux or a phone
it explains that the file is for Windows and starts nothing.

## After each release

```sh
node website/scripts/sync-release.mjs
```

It reads the latest release of the public `lectus-releases` repo and updates
`assets/data/release.json` plus the version, file name, size, date and download link baked into
the HTML. Then add the release to `changelog/index.html` by hand.

## Before this goes public

The copy describes the next release (guided setup, 7-day trial, one-click installer, translation,
local API, seven models). Version 0.7.1, the current download, does not have those yet. Publish
the site together with that release, not before.

Placeholders (search for `data-placeholder`):

- Pricing section: final price, plans and payment route. The trial wording too.
- Legal: final Privacy Notice and Terms, publisher address and identity details.
- Mac: status and download for the launch version, and the remote dictation line that names the Mac.
- Changelog: the "Next" entry (version, date, list) and the 0.6.0 and 0.5.0 notes.
- Contact address in the footer.
- `assets/img/share.png`: a screenshot for now. Replace with a designed 1200x630 card.
- Domain: add `canonical` and `og:url` tags and a sitemap once the domain exists.

Facts to re-check against the shipped build before launch: the default shortcut (Right Ctrl), the
"72 to 148 ms" figure (docs/benchmarks/2026-10-21-windows-models.md), the list of what goes online,
and every sentence in the privacy section against the final Privacy Notice.

## Hosting

Any static host works (Cloudflare Pages, GitHub Pages, Netlify). Serve the `website` folder as
the site root. Nothing is deployed yet.
