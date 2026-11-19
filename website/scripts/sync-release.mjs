// Refreshes the release facts the site shows: assets/data/release.json and the
// no-JavaScript fallbacks baked into the HTML (version, file name, size, link).
// Run after publishing a release:   node website/scripts/sync-release.mjs
// Reads the public lectus-releases repo. Needs no token.
import { readFile, writeFile, readdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const REPO = 'FCisco95/lectus-releases';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
  headers: { Accept: 'application/vnd.github+json', 'User-Agent': 'release-sync' },
});
if (!res.ok) throw new Error(`GitHub answered ${res.status} for the latest release of ${REPO}`);
const rel = await res.json();
const version = rel.tag_name.replace(/^v/, '');
const setup = rel.assets.find((a) => /_x64-setup\.exe$/.test(a.name));
if (!setup) throw new Error(`Release ${rel.tag_name} has no *_x64-setup.exe asset`);

const data = {
  version,
  date: rel.published_at.slice(0, 10),
  windows: { file: setup.name, url: setup.browser_download_url, size_bytes: setup.size },
  notes_url: rel.html_url,
};
await writeFile(path.join(root, 'assets/data/release.json'), JSON.stringify(data, null, 2) + '\n');

const date = new Date(data.date + 'T12:00:00Z').toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' });
const values = { version, file: setup.name, size: `${Math.round(setup.size / 1048576)} MB`, date };

async function* htmlFiles(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory() && entry.name !== 'assets' && entry.name !== 'scripts') yield* htmlFiles(full);
    else if (entry.name.endsWith('.html')) yield full;
  }
}
let touched = 0;
for await (const file of htmlFiles(root)) {
  const before = await readFile(file, 'utf8');
  let after = before.replace(/(data-release="(version|file|size|date)"[^>]*>)[^<]*</g, (m, open, key) => `${open}${values[key]}<`);
  after = after.replace(/(<a\b[^>]*data-dl="win"[^>]*href=")[^"]*"/g, `$1${setup.browser_download_url}"`);
  after = after.replace(/(<a\b[^>]*href=")[^"]*("[^>]*data-dl="win")/g, `$1${setup.browser_download_url}$2`);
  if (after !== before) { await writeFile(file, after); touched += 1; }
}
console.log(`Lectus ${version} (${date}), ${values.size}. Updated release.json and ${touched} page(s).`);
