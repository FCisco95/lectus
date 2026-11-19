// Download page: start the download when the visitor arrived from a download
// button on a Windows PC, explain the situation on anything else, and run the
// first-run walkthrough.
import { os, release } from './site.js';

const status = document.querySelector('[data-dl-status]');
const other = document.querySelector('[data-dl-other]');
const otherText = document.querySelector('[data-dl-other-text]');
const button = document.querySelector('[data-dl-button]');
const wanted = new URLSearchParams(location.search).get('dl') === '1';

const messages = {
  mac: 'You are on a Mac. This file is for Windows. The Mac version is still being finished: see the Mac section below.',
  linux: 'You are on Linux. Lectus runs on Windows today, so this file will not open here.',
  android: 'Lectus is a desktop app. Open this page on your Windows PC to install it.',
  ios: 'Lectus is a desktop app. Open this page on your Windows PC to install it.',
};

if (os !== 'windows' && messages[os]) {
  otherText.textContent = messages[os];
  other.hidden = false;
} else if (wanted && os === 'windows') {
  release.then((rel) => {
    const url = rel?.windows?.url || button.href;
    status.hidden = false;
    // Drop ?dl=1 so a refresh or a shared link does not start a second download.
    history.replaceState(null, '', location.pathname + location.hash);
    setTimeout(() => { location.assign(url); }, 700);
  });
}
button?.addEventListener('click', () => { if (os === 'windows') status.hidden = false; });

// ---- First run walkthrough
const steps = [...document.querySelectorAll('[data-wizard-steps] button')];
const pane = document.querySelector('[data-wizard-pane]');
const dots = [...document.querySelectorAll('.wz-dots i')];
const frame = document.querySelector('[data-demo="wizard"]');
const panes = [
  ['Talk. Lectus types.', 'Tell Lectus which language you speak: English, Portuguese or both. It uses your answer to pick a speech model.', '<span class="on">English</span><span>Portuguese</span><span>Both</span>'],
  ['Privacy and terms', 'Read and accept the terms. While you read, the speech model starts downloading in the background.', '<span class="bar"></span><span>Downloading speech model</span>'],
  ['Microphone', 'Choose your microphone, allow access and say a test sentence. Lectus shows you what it heard.', '<span class="on">Headset microphone</span><span>Show what I said</span>'],
  ['Your words', 'Optional: point Lectus at a folder of your documents and it suggests names and terms to learn.', '<span>Choose a folder</span><span>Skip for now</span>'],
  ['Shortcut', 'Keep Right Ctrl or pick your own key, then try it once before you finish.', '<kbd>Right Ctrl</kbd><span>Change</span>'],
  ['Free trial', 'Start your seven days. No card and no account.', '<span class="on">Start my free trial</span>'],
];
if (steps.length && pane) {
  let index = 0;
  let auto = null;
  const show = (i, animate = true) => {
    index = i;
    const [title, text, extra] = panes[i];
    pane.innerHTML = `<h3></h3><p></p><div class="wz-extra">${extra}</div>`;
    pane.querySelector('h3').textContent = title;
    pane.querySelector('p').textContent = text;
    steps.forEach((b, k) => { if (k === i) b.setAttribute('aria-current', 'step'); else b.removeAttribute('aria-current'); });
    dots.forEach((d, k) => d.classList.toggle('on', k <= i));
    if (animate) { pane.classList.remove('is-fresh'); void pane.offsetWidth; pane.classList.add('is-fresh'); }
  };
  const stopAuto = () => { clearInterval(auto); auto = null; };
  steps.forEach((b, i) => b.addEventListener('click', () => { stopAuto(); show(i); }));
  const calm = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches || document.documentElement.classList.contains('motion-off');
  new IntersectionObserver(([entry]) => {
    if (entry.isIntersecting && !auto && !calm() && index === 0) {
      let rounds = 0;
      auto = setInterval(() => {
        if (calm() || document.hidden) return;
        show((index + 1) % panes.length);
        if (index === panes.length - 1 && ++rounds >= 1) stopAuto();
      }, 3200);
    } else if (!entry.isIntersecting) stopAuto();
  }, { threshold: 0.4 }).observe(frame);
  document.addEventListener('lectus:motion', (e) => { if (e.detail.off) stopAuto(); });
}
