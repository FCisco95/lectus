// Lectus website: the product demos.
// Every demo starts from finished, readable markup. Scripts only replay it.
// A demo runs while it is on screen, the tab is visible and motion is allowed.

const html = document.documentElement;
const reduced = window.matchMedia('(prefers-reduced-motion: reduce)');
const motionOff = () => html.classList.contains('motion-off');
const still = () => reduced.matches || motionOff();

// ---- A clock that only advances while its demo may move
function createClock() {
  const state = { visible: false, user: false };
  const paused = () => !state.visible || state.user || motionOff();
  return {
    state,
    paused,
    sleep(ms, signal) {
      return new Promise((resolve, reject) => {
        let left = ms;
        let last = performance.now();
        const tick = (now) => {
          if (signal?.aborted) return reject(new DOMException('Aborted', 'AbortError'));
          const dt = Math.min(now - last, 60);
          last = now;
          if (!paused()) left -= dt;
          if (left <= 0) resolve(); else requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
      });
    },
  };
}
const swallowAbort = (e) => { if (e?.name !== 'AbortError') throw e; };

// ---- The pill
function createPill(el) {
  const bars = [...el.querySelectorAll('.pill-wave span')];
  const label = el.querySelector('[data-pill-label]');
  const n = bars.length;
  let level = 0;
  let raf = 0;
  let last = 0;
  const frame = (t) => {
    raf = requestAnimationFrame(frame);
    if (t - last < 55) return;
    last = t;
    level = Math.max(level * 0.84, 0.1);
    bars.forEach((bar, i) => {
      const bias = 1 - Math.abs(i - (n - 1) / 2) / n;
      const s = Math.min(1, 0.16 + level * bias * (0.5 + Math.random() * 0.8));
      bar.style.setProperty('--s', s.toFixed(2));
    });
  };
  return {
    set(state) {
      cancelAnimationFrame(raf);
      bars.forEach((bar) => bar.style.removeProperty('--s'));
      el.dataset.state = state;
      if (state === 'recording') { label.textContent = 'Listening…'; level = 0.3; raf = requestAnimationFrame(frame); }
      if (state === 'transcribing') label.textContent = 'Transcribing…';
    },
    kick() { level = 0.6 + Math.random() * 0.4; },
  };
}

function sayWord(heard, word) {
  const span = document.createElement('span');
  span.className = 'w';
  span.textContent = word + ' ';
  heard.append(span);
}
const wordPause = (w) => 130 + w.length * 36 + (/[.,?!]$/.test(w) ? 230 : 0);

function land(target, text) {
  target.textContent = text;
  target.classList.remove('is-fresh');
  void target.offsetWidth;
  target.classList.add('is-fresh');
}

// ---- Hero: one sentence, four apps
function heroDemo(root, clock) {
  const pill = createPill(root.querySelector('[data-pill]'));
  const heard = root.querySelector('[data-heard]');
  const keys = root.querySelector('[data-keys]');
  const who = root.querySelector('[data-who]');
  const whoLabel = root.querySelector('[data-who-label]');
  const pauseBtn = root.querySelector('[data-demo-pause]');
  const buttons = [...root.querySelectorAll('[data-scene-btn]')];
  const scenes = [...root.querySelectorAll('[data-scene]')].map((win) => {
    const target = win.querySelector('[data-land]');
    return { win, id: win.dataset.scene, say: win.dataset.say, lang: win.dataset.lang, target, text: target.textContent };
  });
  let ctrl = null;
  let index = 0;

  const show = (i) => {
    index = i;
    scenes.forEach((s, k) => { s.win.hidden = k !== i; });
    buttons.forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.sceneBtn === scenes[i].id)));
    whoLabel.textContent = scenes[i].lang ? `You say, in ${scenes[i].lang}` : 'You say';
  };
  const rest = (i) => {
    ctrl?.abort();
    show(i);
    const s = scenes[i];
    s.target.textContent = s.text;
    heard.textContent = s.say;
    pill.set('idle');
    keys.classList.remove('is-down');
    who.classList.remove('is-live');
  };

  async function play(i) {
    ctrl?.abort();
    ctrl = new AbortController();
    const { signal } = ctrl;
    show(i);
    const s = scenes[i];
    s.target.textContent = '';
    s.target.classList.remove('is-fresh');
    heard.textContent = '';
    pill.set('idle');
    keys.classList.remove('is-down');
    who.classList.remove('is-live');
    try {
      await clock.sleep(800, signal);
      keys.classList.add('is-down');
      who.classList.add('is-live');
      pill.set('recording');
      await clock.sleep(380, signal);
      for (const word of s.say.split(' ')) {
        sayWord(heard, word);
        pill.kick();
        await clock.sleep(wordPause(word), signal);
      }
      await clock.sleep(260, signal);
      keys.classList.remove('is-down');
      who.classList.remove('is-live');
      pill.set('transcribing');
      await clock.sleep(620, signal);
      pill.set('idle');
      land(s.target, s.text);
      await clock.sleep(3400, signal);
    } catch (e) { return swallowAbort(e); }
    play((i + 1) % scenes.length);
  }

  buttons.forEach((b) => b.addEventListener('click', () => {
    const i = scenes.findIndex((s) => s.id === b.dataset.sceneBtn);
    if (still()) rest(i); else play(i);
  }));
  pauseBtn.addEventListener('click', () => {
    clock.state.user = !clock.state.user;
    pauseBtn.setAttribute('aria-pressed', String(clock.state.user));
    pauseBtn.querySelector('.visually-hidden').textContent = clock.state.user ? 'Play the demo' : 'Pause the demo';
  });

  if (still()) rest(0);
  let started = false;
  return {
    enter() { if (!started && !still()) { started = true; play(0); } },
    motion(off) { if (off) rest(index); else if (!reduced.matches) { started = true; play(index); } },
  };
}

// ---- Typing against speaking
function raceDemo(root, clock) {
  const text = root.dataset.text;
  const words = text.split(' ');
  const SPEED = 8;
  const typeSecs = (words.length / 40) * 60;
  const sentences = text.match(/[^.]+\./g).map((s) => s.trim());
  let spoken = 0;
  const voiceMarks = sentences.map((s) => {
    spoken += (s.split(' ').length / 150) * 60;
    return { at: spoken + 0.4, upto: text.indexOf(s) + s.length };
  });
  const voiceSecs = voiceMarks.at(-1).at;
  const lane = (name) => {
    const el = root.querySelector(`[data-lane="${name}"]`);
    return { el, out: el.querySelector('[data-out]'), clock: el.querySelector('[data-clock]'), bar: el.querySelector('[data-bar]'), text: el.querySelector('.lane-text') };
  };
  const type = lane('type');
  const voice = lane('voice');
  const mmss = (s) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;
  for (const l of [type, voice]) l.text.style.minHeight = `${l.text.offsetHeight}px`;
  let ctrl = null;

  const finish = () => {
    type.out.textContent = text; voice.out.textContent = text;
    type.clock.textContent = mmss(typeSecs); voice.clock.textContent = mmss(voiceSecs);
    type.bar.style.setProperty('--p', 1); voice.bar.style.setProperty('--p', 1);
    type.el.classList.add('is-done'); voice.el.classList.add('is-done');
  };
  finish();

  async function run() {
    ctrl?.abort();
    ctrl = new AbortController();
    const { signal } = ctrl;
    for (const l of [type, voice]) { l.out.textContent = ''; l.el.classList.remove('is-done'); l.bar.style.setProperty('--p', 0); l.clock.textContent = '0:00'; }
    let sim = 0;
    let landed = 0;
    try {
      await clock.sleep(500, signal);
      while (sim < typeSecs) {
        await clock.sleep(40, signal);
        sim = Math.min(typeSecs, sim + 0.04 * SPEED);
        const chars = Math.round((sim / typeSecs) * text.length);
        type.out.textContent = text.slice(0, chars);
        type.bar.style.setProperty('--p', (sim / typeSecs).toFixed(3));
        type.clock.textContent = mmss(sim);
        if (sim < voiceSecs) {
          voice.clock.textContent = mmss(sim);
          voice.bar.style.setProperty('--p', (sim / voiceSecs).toFixed(3));
        }
        while (landed < voiceMarks.length && sim >= voiceMarks[landed].at) {
          voice.out.textContent = text.slice(0, voiceMarks[landed].upto);
          landed += 1;
          if (landed === voiceMarks.length) {
            voice.el.classList.add('is-done');
            voice.clock.textContent = mmss(voiceSecs);
            voice.bar.style.setProperty('--p', 1);
          }
        }
      }
      finish();
    } catch (e) { swallowAbort(e); }
  }
  document.querySelector('[data-race-replay]')?.addEventListener('click', () => { if (still()) finish(); else run(); });
  let started = false;
  return {
    enter() { if (!started && !still()) { started = true; run(); } },
    motion(off) { if (off) { ctrl?.abort(); finish(); } },
  };
}

// ---- Hold the key yourself
function tryDemo(root, clock) {
  const pill = createPill(root.querySelector('[data-pill]'));
  const heard = root.querySelector('[data-heard]');
  const target = root.querySelector('[data-land]');
  const hint = root.querySelector('[data-try-hint]');
  const key = root.querySelector('[data-hold]');
  const keyHint = root.querySelector('[data-hold-hint]');
  const who = root.querySelector('[data-who]');
  const script = ('This is what dictation feels like. I hold the key, I say what I mean, and the words show up where my cursor is. ' +
    'There is no window to switch to and nothing to paste. When I let go, the sentence is already there. ' +
    'Short notes take a breath, and long ones take as long as it takes to say them.').split(' ');
  let pos = 0;
  let holding = false;
  let busy = false;
  let timer = 0;
  let said = [];

  const speak = () => {
    const word = script[pos % script.length];
    pos += 1;
    said.push(word);
    sayWord(heard, word);
    pill.kick();
    timer = setTimeout(speak, wordPause(word));
  };
  const start = () => {
    if (holding || busy) return;
    holding = true;
    said = [];
    heard.textContent = '';
    hint.hidden = true;
    key.classList.add('is-down');
    key.classList.remove('is-nudge');
    keyHint.textContent = 'Listening, keep holding';
    who.classList.add('is-live');
    pill.set('recording');
    timer = setTimeout(speak, 320);
  };
  const stop = async () => {
    if (!holding) return;
    holding = false;
    clearTimeout(timer);
    key.classList.remove('is-down');
    who.classList.remove('is-live');
    if (!said.length) {
      pill.set('idle');
      keyHint.textContent = 'Hold it a little longer';
      return;
    }
    busy = true;
    keyHint.textContent = 'Let go: transcribing';
    pill.set('transcribing');
    await new Promise((r) => setTimeout(r, 560));
    let chunk = said.join(' ').replace(/[,;:]$/, '');
    chunk = chunk[0].toUpperCase() + chunk.slice(1);
    if (!/[.?!]$/.test(chunk)) chunk += '.';
    if (target.textContent.length > 380) target.textContent = '';
    target.querySelectorAll('.is-fresh').forEach((el) => el.classList.remove('is-fresh'));
    const span = document.createElement('span');
    span.className = 'landed is-fresh';
    span.textContent = (target.textContent ? ' ' : '') + chunk;
    target.append(span);
    pill.set('idle');
    heard.textContent = '';
    keyHint.textContent = 'Press and hold again';
    busy = false;
  };

  key.addEventListener('pointerdown', (e) => {
    if (e.button !== 0) return;
    try { key.setPointerCapture(e.pointerId); } catch { /* synthetic pointers cannot be captured */ }
    start();
  });
  key.addEventListener('pointerup', stop);
  key.addEventListener('pointercancel', stop);
  key.addEventListener('lostpointercapture', stop);
  key.addEventListener('contextmenu', (e) => e.preventDefault());
  key.addEventListener('keydown', (e) => { if ((e.key === ' ' || e.key === 'Enter') && !e.repeat) { e.preventDefault(); start(); } });
  key.addEventListener('keyup', (e) => { if (e.key === ' ' || e.key === 'Enter') stop(); });
  key.addEventListener('blur', stop);
  // The real key, while the demo is on screen.
  window.addEventListener('keydown', (e) => { if (e.code === 'ControlRight' && !e.repeat && clock.state.visible) start(); });
  window.addEventListener('keyup', (e) => { if (e.code === 'ControlRight') stop(); });
  window.addEventListener('blur', stop);

  let nudged = false;
  return {
    enter() { if (!nudged && !still()) { nudged = true; key.classList.add('is-nudge'); } },
    leave() { stop(); },
  };
}

// ---- Clean-up switch
function cleanupDemo(root, clock) {
  const sw = root.querySelector('[data-cleanup-switch]');
  const text = root.querySelector('[data-cleanup-text]');
  const foot = root.querySelector('[data-cleanup-foot]');
  const onNote = foot.textContent;
  const set = (on) => {
    sw.setAttribute('aria-checked', String(on));
    text.classList.toggle('is-on', on);
    foot.textContent = on ? onNote : 'Off: the transcript exactly as it was spoken.';
  };
  let touched = false;
  sw.addEventListener('click', () => { touched = true; set(sw.getAttribute('aria-checked') !== 'true'); });
  let started = false;
  return {
    async enter() {
      if (started || still()) return;
      started = true;
      set(false);
      try { await clock.sleep(1500); } catch { return; }
      if (!touched) set(true);
    },
  };
}

// ---- Vocabulary
function vocabDemo(root, clock) {
  const row = root.querySelector('.vocab-live');
  const heard = root.querySelector('[data-vocab-heard]');
  const written = root.querySelector('[data-vocab-written]');
  const pairs = [
    ['cube control get pods', 'kubectl get pods'],
    ['send it to marta raise', 'Send it to Marta Reis'],
    ['open lek tus settings', 'Open Lectus settings'],
    ['back up the post gress database', 'Back up the Postgres database'],
  ];
  let i = 0;
  let ctrl = null;
  async function loop() {
    ctrl?.abort();
    ctrl = new AbortController();
    const { signal } = ctrl;
    try {
      for (;;) {
        await clock.sleep(2600, signal);
        i = (i + 1) % pairs.length;
        row.classList.add('is-swapping');
        await clock.sleep(260, signal);
        heard.textContent = pairs[i][0];
        written.textContent = pairs[i][1];
        row.classList.add('is-waiting');
        row.classList.remove('is-swapping');
        await clock.sleep(950, signal);
        row.classList.remove('is-waiting');
      }
    } catch (e) { swallowAbort(e); }
  }
  let started = false;
  return {
    enter() { if (!started && !still()) { started = true; loop(); } },
    motion(off) { if (off) { ctrl?.abort(); row.classList.remove('is-swapping', 'is-waiting'); } else if (!reduced.matches) loop(); },
  };
}

// ---- Languages
function langDemo(root, clock) {
  const tag = root.querySelector('[data-lang-tag]');
  const target = root.querySelector('[data-land]');
  const items = [...root.querySelectorAll('.lang-list li')];
  const lines = [
    ['English', 'en', 'See you at three.'],
    ['Portuguese', 'pt', 'Vemo-nos às três.'],
    ['Spanish', 'es', 'Nos vemos a las tres.'],
    ['French', 'fr', 'On se voit à trois heures.'],
    ['German', 'de', 'Wir sehen uns um drei.'],
    ['Italian', 'it', 'Ci vediamo alle tre.'],
    ['Dutch', 'nl', 'We zien elkaar om drie uur.'],
    ['Japanese', 'ja', '3時に会いましょう。'],
    ['Chinese', 'zh', '三点见。'],
  ];
  let i = 0;
  let ctrl = null;
  async function loop() {
    ctrl?.abort();
    ctrl = new AbortController();
    const { signal } = ctrl;
    try {
      for (;;) {
        await clock.sleep(2400, signal);
        i = (i + 1) % lines.length;
        const [name, code, line] = lines[i];
        tag.textContent = name;
        target.lang = code;
        land(target, line);
        items.forEach((li) => li.classList.toggle('on', li.dataset.l === name));
      }
    } catch (e) { swallowAbort(e); }
  }
  let started = false;
  return {
    enter() { if (!started && !still()) { started = true; loop(); } },
    motion(off) { if (off) ctrl?.abort(); else if (!reduced.matches) loop(); },
  };
}

// ---- Two computers
function remoteDemo(root, clock) {
  const pill = createPill(root.querySelector('[data-pill]'));
  const target = root.querySelector('[data-land]');
  const lines = ['The draft is ready for your comments.', 'Can you send me the photos from Saturday?', 'I will call you back in ten minutes.'];
  let i = 0;
  let ctrl = null;
  const rest = () => { ctrl?.abort(); pill.set('idle'); root.dataset.phase = 'land'; target.textContent = lines[i]; };
  async function loop() {
    ctrl?.abort();
    ctrl = new AbortController();
    const { signal } = ctrl;
    try {
      for (;;) {
        root.dataset.phase = 'speak';
        target.textContent = '';
        target.classList.remove('is-fresh');
        pill.set('recording');
        for (const word of lines[i].split(' ')) { pill.kick(); await clock.sleep(wordPause(word), signal); }
        pill.set('transcribing');
        await clock.sleep(520, signal);
        pill.set('idle');
        root.dataset.phase = 'send';
        await clock.sleep(950, signal);
        root.dataset.phase = 'land';
        land(target, lines[i]);
        await clock.sleep(3000, signal);
        root.dataset.phase = 'reset';
        i = (i + 1) % lines.length;
      }
    } catch (e) { swallowAbort(e); }
  }
  rest();
  let started = false;
  return {
    enter() { if (!started && !still()) { started = true; loop(); } },
    motion(off) { if (off) rest(); else if (!reduced.matches) loop(); },
  };
}

// ---- Demos that are plain CSS loops only need to know when they are on screen
const cssOnly = () => ({});

const factories = { hero: heroDemo, race: raceDemo, try: tryDemo, cleanup: cleanupDemo, vocab: vocabDemo, lang: langDemo, remote: remoteDemo, flow: cssOnly, install: cssOnly, term: cssOnly, steps: cssOnly };
const live = new Map();
for (const el of document.querySelectorAll('[data-demo]')) {
  const make = factories[el.dataset.demo];
  if (!make) continue;
  const clock = createClock();
  live.set(el, { clock, api: make(el, clock) || {} });
}
const io = new IntersectionObserver((entries) => {
  for (const entry of entries) {
    const item = live.get(entry.target);
    if (!item) continue;
    item.clock.state.visible = entry.isIntersecting;
    entry.target.classList.toggle('is-inview', entry.isIntersecting);
    if (entry.isIntersecting) item.api.enter?.(); else item.api.leave?.();
  }
}, { threshold: 0.3 });
live.forEach((_, el) => io.observe(el));
document.addEventListener('lectus:motion', (e) => live.forEach((item) => item.api.motion?.(e.detail.off)));
