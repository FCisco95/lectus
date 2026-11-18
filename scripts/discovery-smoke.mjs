/** Spotlight and "Get started" fixtures for scripts/ui-smoke.mjs.
 * Run: UI_SMOKE_DISCOVERY=1 node scripts/ui-smoke.mjs
 * Browser fixture only: IPC is mocked by ui-smoke.mjs installFixture.
 */
import assert from 'node:assert/strict';
import path from 'node:path';

const ALL = ['vocabulary', 'dictations', 'models', 'ai-cleanup'];
const TITLES = {
  vocabulary: 'Teach Lectus your words',
  dictations: 'Find past dictations',
  models: 'Choose your speech model',
  'ai-cleanup': 'Polish and translate',
};
const STORE = 'discovery-fixture';

// installFixture rebuilds window.fixture on every navigation; this runs after
// it and re-applies the state a test saved, so a reload acts like a restart.
function restoreFixture(key) {
  const raw = sessionStorage.getItem(key);
  if (!raw) return;
  const saved = JSON.parse(raw);
  Object.assign(window.fixture.config, saved.config || {});
  if (saved.history) window.fixture.history = saved.history;
  if (saved.autostart !== undefined) window.fixture.autostart = saved.autostart;
  if (saved.whatsNew) window.fixture.whatsNew = saved.whatsNew;
  if (saved.tryBanner) localStorage.setItem('lectus.try-dictation', '1');
}

const home = page => page.getByRole('heading', { name: 'Welcome back, Alex', exact: true }).waitFor({ timeout: 20000 });
const dialog = (page, id) => page.getByRole('dialog', { name: TITLES[id], exact: true });
const anySpotlight = page => page.locator('.spotlight');
const settled = page => page.waitForFunction(() => {
  const pop = document.querySelector('.spotlight');
  return pop && pop.getAnimations().length === 0;
});
const savedSeen = (page, id) => page.waitForFunction(id => fixture.config.seen_spotlights?.includes(id)
  && fixture.saveRequests.some(request => request.seen_spotlights?.includes(id)), id);

/** Load the shell with these fixture overrides, as if the app had just started. */
async function boot(page, state) {
  await page.addInitScript(restoreFixture, STORE);
  await page.evaluate(([key, value]) => sessionStorage.setItem(key, JSON.stringify(value)), [STORE, state]);
  await page.reload();
  await home(page);
}

/** Keep what the app saved (config) and restart. */
async function restart(page, extra = {}) {
  await page.evaluate(([key, extra]) => {
    const saved = JSON.parse(sessionStorage.getItem(key) || '{}');
    sessionStorage.setItem(key, JSON.stringify({ ...saved, whatsNew: null, tryBanner: false, ...extra, config: window.fixture.config }));
  }, [STORE, extra]);
  await page.reload();
  await home(page);
}

/** Popover, arrow and anchor rectangles in viewport pixels. */
function geometry(page, anchor) {
  return page.evaluate(anchor => {
    const rect = el => el.getBoundingClientRect().toJSON();
    const pop = document.querySelector('.spotlight');
    const arrow = rect(pop.querySelector('.spotlight-arrow'));
    return {
      pop: rect(pop), item: rect(document.querySelector(anchor)), arrowY: arrow.top + arrow.height / 2,
      vw: innerWidth, vh: innerHeight, scroll: document.documentElement.scrollWidth,
      appTop: document.querySelector('.settings-app').getBoundingClientRect().top,
      clipped: [...pop.querySelectorAll('.spotlight-title, .spotlight-body, .btn')].some(el => el.scrollWidth > el.clientWidth + 1),
    };
  }, anchor);
}

function assertAnchored(g, label) {
  const itemMid = g.item.top + g.item.height / 2;
  assert(g.pop.left >= g.item.right + 6 && g.pop.left <= g.item.right + 24, `${label}: popover sits right of the item (${g.pop.left} vs ${g.item.right})`);
  assert(Math.abs(g.arrowY - itemMid) <= 2, `${label}: arrow points at the item (${g.arrowY} vs ${itemMid})`);
  assert(g.pop.top >= g.appTop && g.pop.bottom <= g.vh && g.pop.left >= 0 && g.pop.right <= g.vw, `${label}: popover inside the window, below the titlebar`);
  assert(g.scroll <= g.vw, `${label}: no horizontal page overflow`);
  assert(!g.clipped, `${label}: text and button not clipped`);
}

const noSpotlightFor = async (page, ms) => { await page.waitForTimeout(ms); assert.equal(await anySpotlight(page).count(), 0, 'no spotlight expected'); };
const card = page => page.getByRole('region', { name: 'Get started', exact: true });
const step = (page, id) => card(page).locator(`[data-step="${id}"]`);
const isDone = (page, id) => step(page, id).evaluate(el => el.classList.contains('done'));

export async function runDiscoverySmoke({ checked, pageFixture, output, results }) {
  await checked('No spotlight before the first dictation; history-added brings the first one', async page => {
    await boot(page, { config: { seen_spotlights: [] }, history: [] });
    await noSpotlightFor(page, 2200);
    assert.equal(await isDone(page, 'dictate'), false);
    await page.evaluate(() => {
      fixture.history.push({ text: 'First fixture dictation.', timestamp: 1792573300000, language: 'en' });
      fixture.emit('history-added', fixture.history.at(-1));
    });
    await dialog(page, 'vocabulary').waitFor();
    assert.equal(await isDone(page, 'dictate'), true);
  });

  await checked('Vocabulary spotlight anchors beside its item; its button opens Vocabulary and saves the id', async page => {
    await boot(page, { config: { seen_spotlights: [] } });
    const pop = dialog(page, 'vocabulary');
    await pop.waitFor(); await settled(page);
    assert.equal(await pop.getAttribute('aria-modal'), 'true');
    assert.equal(await page.evaluate(() => document.querySelector('.shell-columns').inert), true, 'shell inert under the spotlight');
    assert.equal(await page.evaluate(() => document.activeElement?.textContent), 'Open Vocabulary', 'primary button focused');
    assertAnchored(await geometry(page, '[data-surface="vocabulary"]'), 'vocabulary 1100x800');
    await page.getByRole('button', { name: 'Open Vocabulary', exact: true }).click();
    await page.getByRole('heading', { name: 'Dictionary', exact: true }).waitFor();
    assert.equal(await page.locator('[data-surface="vocabulary"]').getAttribute('aria-current'), 'page');
    await savedSeen(page, 'vocabulary');
    await page.waitForFunction(() => document.activeElement?.dataset.surface === 'vocabulary');
    await noSpotlightFor(page, 2200);
  });

  await checked('Close, Escape, backdrop and the lit item each persist; one per session, restart shows the next', async page => {
    await boot(page, { config: { seen_spotlights: [] } });
    await dialog(page, 'vocabulary').waitFor();
    await page.getByRole('button', { name: 'Close tip', exact: true }).click();
    await savedSeen(page, 'vocabulary');
    await page.waitForFunction(() => document.activeElement?.dataset.surface === 'vocabulary');
    assert.equal(await page.getByRole('heading', { name: 'Welcome back, Alex', exact: true }).count(), 1, 'close does not navigate');
    await noSpotlightFor(page, 2200);

    await restart(page);
    await dialog(page, 'dictations').waitFor(); await settled(page);
    await page.keyboard.press('Escape');
    await savedSeen(page, 'dictations');
    assert.equal(await anySpotlight(page).count(), 0);
    await page.waitForFunction(() => document.activeElement?.dataset.surface === 'dictations');
    await noSpotlightFor(page, 2200);

    await restart(page);
    await dialog(page, 'models').waitFor(); await settled(page);
    assertAnchored(await geometry(page, '.sidebar [data-settings-trigger]'), 'models anchor');
    await page.mouse.click(900, 400);
    await savedSeen(page, 'models');
    assert.equal(await page.getByRole('dialog', { name: 'Settings', exact: true }).count(), 0, 'backdrop click only dismisses');

    await restart(page);
    await dialog(page, 'ai-cleanup').waitFor(); await settled(page);
    await page.locator('.spotlight-halo').click();
    await page.getByRole('dialog', { name: 'Settings', exact: true }).getByRole('heading', { name: 'AI cleanup', exact: true }).waitFor();
    await savedSeen(page, 'ai-cleanup');
    await page.keyboard.press('Escape');

    await restart(page);
    assert.deepEqual(await page.evaluate(() => fixture.config.seen_spotlights), ALL);
    await noSpotlightFor(page, 2200);
  });

  await checked('Settings-tab spotlights anchor to Settings and open the modal at their tab', async page => {
    await boot(page, { config: { seen_spotlights: ['vocabulary', 'dictations'] } });
    await dialog(page, 'models').waitFor(); await settled(page);
    assertAnchored(await geometry(page, '.sidebar [data-settings-trigger]'), 'models 1100x800');
    await page.getByRole('button', { name: 'Open Models', exact: true }).click();
    const settingsDialog = page.getByRole('dialog', { name: 'Settings', exact: true });
    await settingsDialog.getByRole('heading', { name: 'Models', exact: true }).waitFor();
    assert.equal(await settingsDialog.getByRole('button', { name: 'Models', exact: true }).getAttribute('aria-current'), 'page');
    await savedSeen(page, 'models');
    await page.keyboard.press('Escape');
    await restart(page);
    await dialog(page, 'ai-cleanup').waitFor();
    await page.getByRole('button', { name: 'Open AI cleanup', exact: true }).click();
    await settingsDialog.getByRole('heading', { name: 'AI cleanup', exact: true }).waitFor();
    assert.equal(await settingsDialog.getByRole('button', { name: 'AI cleanup', exact: true }).getAttribute('aria-current'), 'page');
    await savedSeen(page, 'ai-cleanup');
  });

  await checked("Never shown over What's new or Settings; waits for them to close", async page => {
    await boot(page, { config: { seen_spotlights: [] }, whatsNew: { version: '0.7.2', notes: 'Fixture release notes.' } });
    await page.getByRole('dialog', { name: "What's new in Lectus 0.7.2", exact: true }).waitFor();
    await noSpotlightFor(page, 2200);
    await page.getByRole('button', { name: 'Got it', exact: true }).click();
    await dialog(page, 'vocabulary').waitFor();

    await restart(page, { history: [] });
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await page.evaluate(() => {
      fixture.history.push({ text: 'Dictated while Settings was open.', timestamp: 1792573300000, language: 'en' });
      fixture.emit('history-added', fixture.history.at(-1));
    });
    await noSpotlightFor(page, 2200);
    await page.keyboard.press('Escape');
    await dialog(page, 'vocabulary').waitFor();
  });

  await checked('The post-setup "Try it now" banner holds spotlights back until it is gone', async page => {
    await boot(page, { config: { seen_spotlights: [] }, tryBanner: true });
    await page.getByText(/^Try it now:/).waitFor();
    await noSpotlightFor(page, 2600);
    await page.getByRole('button', { name: 'Dismiss', exact: true }).click();
    await dialog(page, 'vocabulary').waitFor({ timeout: 6000 });
  });

  await checked('Get started reflects real state, its steps navigate and Hide persists', async page => {
    await boot(page, { config: { seen_spotlights: ALL, dictionary_words: [] }, autostart: false });
    await card(page).waitFor();
    assert.equal(await card(page).locator('.gs-count').innerText(), '1 of 3');
    assert.deepEqual([await isDone(page, 'dictate'), await isDone(page, 'vocabulary'), await isDone(page, 'autostart')], [true, false, false]);
    assert.equal(await card(page).getByRole('button', { name: 'Dictate once (done)', exact: true }).count(), 1);

    await page.getByRole('button', { name: 'Dictations', exact: true }).click();
    await step(page, 'dictate').click();
    await home(page);

    await step(page, 'vocabulary').click();
    await page.getByRole('heading', { name: 'Dictionary', exact: true }).waitFor();
    const word = page.getByRole('textbox', { name: 'Custom words', exact: true });
    await word.fill('Eclectus'); await word.press('Enter');
    await page.waitForFunction(() => document.querySelector('.gs-count')?.textContent === '2 of 3');
    assert.equal(await isDone(page, 'vocabulary'), true);

    await step(page, 'autostart').click();
    const settingsDialog = page.getByRole('dialog', { name: 'Settings', exact: true });
    await settingsDialog.getByRole('heading', { name: 'General', exact: true }).waitFor();
    await page.getByRole('switch', { name: 'Launch at login', exact: true }).click();
    await page.waitForFunction(() => fixture.autostart === true);
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => !document.querySelector('.getting-started'));

    // Hide, from a fresh start with steps left.
    await page.evaluate(() => { fixture.config.dictionary_words = []; });
    await restart(page, { autostart: false });
    await card(page).waitFor();
    await page.getByRole('button', { name: 'Hide Get started', exact: true }).click();
    assert.equal(await card(page).count(), 0);
    await page.waitForFunction(() => fixture.config.getting_started_hidden === true
      && fixture.saveRequests.some(request => request.getting_started_hidden === true));
    await restart(page, { autostart: false });
    await page.waitForTimeout(600);
    assert.equal(await page.locator('.getting-started').count(), 0, 'Hide persists across restart');
  });

  // Screenshots: every spotlight at both window sizes and themes, plus the card.
  let captures = 0;
  for (const [width, height] of [[960, 680], [720, 540]]) for (const theme of ['light', 'dark']) {
    const page = await pageFixture({ width, height });
    try {
      for (const [index, id] of ALL.entries()) {
        const state = { config: { seen_spotlights: ALL.slice(0, index), theme, dictionary_words: [] }, autostart: false };
        if (index === 0) await boot(page, state);
        else {
          await page.evaluate(([key, value]) => sessionStorage.setItem(key, JSON.stringify(value)), [STORE, state]);
          await page.reload(); await home(page);
        }
        await page.waitForFunction(theme => document.documentElement.dataset.theme === theme, theme);
        await dialog(page, id).waitFor(); await settled(page);
        await page.screenshot({ animations: 'disabled', path: path.join(output, `spotlight-${id}-${theme}-${width}.png`) });
        captures++;
        const anchor = id === 'vocabulary' || id === 'dictations' ? `[data-surface="${id}"]` : '.sidebar [data-settings-trigger]';
        assertAnchored(await geometry(page, anchor), `${id} ${theme} ${width}x${height}`);
      }
      await page.keyboard.press('Escape');
      await page.waitForFunction(() => !document.querySelector('.spotlight') && !document.querySelector('.settings-autosave-status'));
      const layout = await page.evaluate(() => {
        const nav = document.querySelector('.sidebar');
        const cardEl = document.querySelector('.getting-started');
        const box = el => el.getBoundingClientRect();
        const visible = !!cardEl && getComputedStyle(cardEl).display !== 'none';
        return {
          visible,
          navOverflow: nav.scrollHeight > nav.clientHeight + 1,
          pageOverflow: document.documentElement.scrollWidth > innerWidth,
          cardClipped: visible && [...cardEl.querySelectorAll('.gs-title, .gs-label, .gs-hide, .gs-count')].some(el => el.scrollWidth > el.clientWidth + 1),
          cardAboveFoot: !visible || box(cardEl).bottom <= box(document.querySelector('.sidebar-foot')).top,
          cardBelowGroup: !visible || box(cardEl).top >= box(document.querySelector('.sidebar-group')).bottom,
          footAtBottom: Math.abs(box(document.querySelector('.sidebar-foot')).bottom - (box(nav).bottom - parseFloat(getComputedStyle(nav).paddingBottom))) <= 1,
        };
      });
      assert.equal(layout.visible, width > 820, `card ${width > 820 ? 'shown' : 'hidden'} at ${width}px`);
      assert.deepEqual(layout, { visible: width > 820, navOverflow: false, pageOverflow: false, cardClipped: false, cardAboveFoot: true, cardBelowGroup: true, footAtBottom: true }, `sidebar layout ${theme} ${width}`);
      await page.screenshot({ animations: 'disabled', path: path.join(output, `getting-started-${theme}-${width}.png`) });
      captures++;
      assert.deepEqual(page.errors, []); assert.deepEqual(page.externalRequests, []);
      assert.deepEqual(await page.evaluate(() => fixture.unknown), []);
    } finally { await page.close(); }
  }
  // A wide but short window drops the card so Settings and Help stay reachable.
  {
    const page = await pageFixture({ width: 1000, height: 560 });
    try {
      await boot(page, { config: { seen_spotlights: ALL, dictionary_words: [] }, autostart: false });
      await page.waitForFunction(() => document.querySelector('.getting-started'));
      const short = await page.evaluate(() => {
        const nav = document.querySelector('.sidebar');
        return { hidden: getComputedStyle(document.querySelector('.getting-started')).display === 'none', navOverflow: nav.scrollHeight > nav.clientHeight + 1 };
      });
      assert.deepEqual(short, { hidden: true, navOverflow: false }, 'card hidden and sidebar fits at 1000x560');
      results.push({ name: 'Get started yields to Settings and Help on a short 1000x560 window', passed: true });
    } finally { await page.close(); }
  }
  results.push({ name: `${captures} discovery light/dark captures at 960x680 and 720x540; anchoring, clipping and sidebar overflow checks`, passed: true });
}
