// Lectus website: shared behaviour. No dependencies, no third-party requests.

const html = document.documentElement;

// ---- Author aid: /?placeholders outlines everything still waiting for real content.
if (new URLSearchParams(location.search).has('placeholders')) html.classList.add('show-placeholders');

// ---- Header
const header = document.querySelector('[data-header]');
if (header) {
  const onScroll = () => header.classList.toggle('is-stuck', window.scrollY > 8);
  onScroll();
  window.addEventListener('scroll', onScroll, { passive: true });
}
const navToggle = document.querySelector('[data-nav-toggle]');
const nav = document.querySelector('[data-nav]');
if (navToggle && nav) {
  const setOpen = (open) => {
    nav.classList.toggle('is-open', open);
    navToggle.setAttribute('aria-expanded', String(open));
    document.body.style.overflow = open ? 'hidden' : '';
  };
  navToggle.addEventListener('click', () => setOpen(navToggle.getAttribute('aria-expanded') !== 'true'));
  nav.addEventListener('click', (e) => { if (e.target.closest('a')) setOpen(false); });
  window.addEventListener('keydown', (e) => { if (e.key === 'Escape') setOpen(false); });
  window.matchMedia('(min-width: 901px)').addEventListener('change', () => setOpen(false));
}

// ---- Which system is the visitor on?
function detectOS() {
  const p = (navigator.userAgentData?.platform || navigator.platform || '').toLowerCase();
  const ua = navigator.userAgent.toLowerCase();
  if (/android/.test(ua)) return 'android';
  if (/iphone|ipad|ipod/.test(ua) || (p.includes('mac') && navigator.maxTouchPoints > 1)) return 'ios';
  if (p.includes('win') || ua.includes('windows')) return 'windows';
  if (p.includes('mac') || ua.includes('mac os')) return 'mac';
  if (p.includes('linux') || ua.includes('linux')) return 'linux';
  return 'other';
}
export const os = detectOS();
html.dataset.os = os;

const osNotes = {
  mac: 'You are on a Mac. Lectus runs on Windows today; the Mac version is still being finished.',
  linux: 'You are on Linux. Lectus runs on Windows today.',
  android: 'Lectus is a desktop app. Open this page on your Windows PC to install it.',
  ios: 'Lectus is a desktop app. Open this page on your Windows PC to install it.',
};
for (const el of document.querySelectorAll('[data-os-note]')) {
  if (osNotes[os]) { el.textContent = osNotes[os]; el.hidden = false; }
}

// ---- Current release. The file is refreshed by scripts/sync-release.mjs at
// release time; the HTML already carries the same values for visitors without JS.
export const release = fetch(new URL('../data/release.json', import.meta.url))
  .then((r) => (r.ok ? r.json() : null))
  .catch(() => null);

const fmtDate = (iso) => new Date(iso + 'T12:00:00Z').toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' });
const fmtSize = (bytes) => `${Math.round(bytes / 1048576)} MB`;

release.then((rel) => {
  if (!rel) return;
  const values = {
    version: rel.version,
    date: rel.date ? fmtDate(rel.date) : '',
    size: rel.windows?.size_bytes ? fmtSize(rel.windows.size_bytes) : '',
    file: rel.windows?.file || '',
  };
  for (const el of document.querySelectorAll('[data-release]')) {
    const v = values[el.dataset.release];
    if (v) el.textContent = v;
  }
  for (const a of document.querySelectorAll('a[data-dl="win"]')) if (rel.windows?.url) a.href = rel.windows.url;
});

// ---- Copy buttons
for (const btn of document.querySelectorAll('[data-copy]')) {
  btn.addEventListener('click', async () => {
    const src = document.querySelector(btn.dataset.copy);
    const label = btn.querySelector('[data-copy-label]') || btn;
    if (!src) return;
    try {
      await navigator.clipboard.writeText(src.textContent.trim());
      label.textContent = 'Copied';
    } catch {
      label.textContent = 'Select and copy';
    }
    setTimeout(() => { label.textContent = 'Copy'; }, 1800);
  });
}

// ---- Site-wide motion switch (footer). Anything that moves on its own obeys it.
const KEY = 'lectus-motion';
const stored = (() => { try { return localStorage.getItem(KEY); } catch { return null; } })();
if (stored === 'off') html.classList.add('motion-off');
for (const btn of document.querySelectorAll('[data-motion-toggle]')) {
  const sync = () => {
    const off = html.classList.contains('motion-off');
    btn.setAttribute('aria-pressed', String(off));
    btn.textContent = off ? 'Play animations' : 'Pause animations';
  };
  sync();
  btn.addEventListener('click', () => {
    const off = html.classList.toggle('motion-off');
    try { localStorage.setItem(KEY, off ? 'off' : 'on'); } catch { /* private mode */ }
    document.dispatchEvent(new CustomEvent('lectus:motion', { detail: { off } }));
    document.querySelectorAll('[data-motion-toggle]').forEach((b) => {
      b.setAttribute('aria-pressed', String(off));
      b.textContent = off ? 'Play animations' : 'Pause animations';
    });
  });
}
