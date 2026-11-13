import assert from 'node:assert/strict';
import path from 'node:path';

export async function openWizard(page, { ready = true, accepted = true, language = 'Both' } = {}) {
  await page.evaluate(({ ready, accepted }) => {
    fixture.wizard = true;
    fixture.wizardDownloads = [];
    fixture.config.onboarding_completed = false;
    fixture.config.sound_alikes = [];
    fixture.legal.ready = ready;
    fixture.legal.accepted = accepted;
    fixture.models = [
      { name: 'ggml-base.bin', display_name: 'Base', size_mb: 142, downloaded: false, active: false, download: { downloading: false, progress: 0, error: null } },
      { name: 'ggml-small.bin', display_name: 'Small', size_mb: 466, downloaded: false, active: false, download: { downloading: false, progress: 0, error: null } },
      { name: 'ggml-large-v3-turbo-q5_0.bin', display_name: 'Large v3 Turbo Compact', size_mb: 574, downloaded: false, active: false, download: { downloading: false, progress: 0, error: null } },
    ];
    fixture.emit('tauri://focus', null);
  }, { ready, accepted });
  await page.getByRole('heading', { name: 'Talk. Lectus types.', exact: true }).waitFor();
  await page.getByRole('radio', { name: language, exact: true }).check();
}

export async function toMic(page, options) {
  await openWizard(page, options);
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('heading', { name: 'Privacy & terms', exact: true }).waitFor();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('heading', { name: 'Check your microphone', exact: true }).waitFor();
}

export async function toTrial(page) {
  await toMic(page);
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('button', { name: /^(Continue|Skip)$/ }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByRole('heading', { name: '7 days free, starting now', exact: true }).waitFor();
}

export async function runWizardSmoke({ checked, pageFixture, output, results }) {
  // Catches a missing language save and a stale snapshot overwriting other settings.
  await checked('Wizard saves Both as auto from a fresh config snapshot', async page => {
    await openWizard(page);
    await page.evaluate(() => { fixture.config.display_name = 'Concurrent name'; });
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page.waitForFunction(() => fixture.saves.length > 0);
    assert.equal(await page.evaluate(() => fixture.config.language), 'auto');
    assert.equal(await page.evaluate(() => fixture.config.display_name), 'Concurrent name');
  });
  await checked('Wizard draft terms cannot be accepted or bypassed', async page => {
    await openWizard(page, { ready: false, accepted: false });
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page.getByText(/These drafts are not a final agreement/).waitFor();
    assert(await page.getByRole('button', { name: 'Continue', exact: true }).isDisabled());
    assert.equal(await page.getByRole('button', { name: 'Accept Terms', exact: true }).count(), 0);
    assert.deepEqual(await page.evaluate(() => fixture.wizardDownloads), []);
  });
  for (const [language, expected] of [['Both', 'ggml-small.bin'], ['English', 'ggml-base.bin']]) {
    await checked(`Wizard downloads and selects ${expected} only after accepted Terms`, async page => {
      await openWizard(page, { language, accepted: false });
      await page.getByRole('button', { name: 'Continue', exact: true }).click();
      await page.getByRole('checkbox', { name: 'I have read and agree to the Terms.', exact: true }).check();
      assert.deepEqual(await page.evaluate(() => fixture.wizardDownloads), []);
      await page.getByRole('button', { name: 'Accept Terms', exact: true }).click();
      await page.waitForFunction(expected => fixture.config.model_name === expected && fixture.wizardDownloads.includes(expected), expected);
      assert.equal(await page.evaluate(() => fixture.calls.includes('wizard_mic_start')), false);
    });
  }
  await checked('Wizard completes without a wallet and Home offers the real shortcut once', async page => {
    await page.evaluate(() => { const old = window.__TAURI_INTERNALS__.invoke; window.__TAURI_INTERNALS__.invoke = (cmd, args) => cmd === 'license_status' ? Promise.resolve({ kind: 'unlinked' }) : old(cmd, args); });
    await toTrial(page);
    await page.getByText(/wallet holding \$20 of ORGANIC or Mycel/).waitFor();
    await page.getByRole('button', { name: 'Start my free trial', exact: true }).click();
    await page.getByRole('heading', { name: 'Welcome back, Alex', exact: true }).waitFor();
    assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'complete_onboarding').length), 1);
    await page.getByText(/Try it now: click into any text field/).waitFor();
    await page.getByRole('button', { name: 'Dismiss', exact: true }).click();
    assert.equal(await page.getByText(/Try it now: click into any text field/).count(), 0);
  });
  for (const width of [960, 720]) for (const theme of ['light', 'dark']) {
    const page = await pageFixture({ width, height: width === 960 ? 680 : 540 });
    try {
      await page.evaluate(theme => { fixture.config.theme = theme; fixture.emit('theme-changed', theme); }, theme);
      await openWizard(page);
      for (const step of ['welcome', 'terms', 'mic', 'words', 'shortcut', 'trial']) {
        await page.locator(`[data-wizard-step="${step}"]`).waitFor();
        await page.screenshot({ animations: 'disabled', path: path.join(output, `${step}-${theme}-${width}.png`) });
        assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `${step} page overflow`);
        assert(await page.locator('.wizard-root').evaluate(el => el.scrollWidth <= el.clientWidth + 1), `${step} content overflow`);
        if (step !== 'trial') await page.getByRole('button', { name: /^(Continue|Skip)$/ }).click();
      }
      assert.deepEqual(page.errors, []);
    } finally { await page.close(); }
  }
  results.push({ name: '24 wizard light/dark screenshots at 960x680 and 720x540 without horizontal overflow', passed: true });
}

export async function runMicSmoke({ checked, output }) {
  await checked('Wizard microphone level follows events and shows its transcript', async page => {
    await toMic(page);
    await page.getByLabel('Input device', { exact: true }).selectOption('Fixture microphone');
    await page.waitForFunction(() => fixture.config.input_device === 'Fixture microphone');
    await page.getByRole('button', { name: 'Test my microphone', exact: true }).click();
    await page.waitForFunction(() => fixture.wizardRecording === true);
    await page.evaluate(() => fixture.emit('wizard-audio-level', .65));
    await page.waitForFunction(() => document.querySelector('[aria-label="Microphone level"]')?.getAttribute('aria-valuenow') === '65');
    await page.screenshot({ path: path.join(output, 'mic-recording.png'), animations: 'disabled' });
    await page.getByRole('button', { name: 'Show what I said', exact: true }).click();
    await page.getByText('Testing Lectus.', { exact: true }).waitFor();
    assert.deepEqual(await page.evaluate(() => fixture.wizardRawRequests), [false]);
    assert.equal(await page.evaluate(() => fixture.wizardRecording), false);
  });
  await checked('Wizard microphone failure offers Set up later', async page => {
    await toMic(page);
    await page.evaluate(() => { fixture.wizardMicError = 'Fixture microphone disconnected'; });
    await page.getByRole('button', { name: 'Test my microphone', exact: true }).click();
    await page.getByRole('alert').filter({ hasText: 'Fixture microphone disconnected' }).waitFor();
    await page.getByRole('button', { name: 'Set up later', exact: true }).click();
    await page.getByRole('heading', { name: 'Your words', exact: true }).waitFor();
  });
  for (const [elapsed, fast] of [[100, true], [1000, false]]) {
    await checked(`Wizard ${fast ? 'fast' : 'slow'} recognition ${fast ? 'offers' : 'does not offer'} the accurate model`, async page => {
      await toMic(page);
      await page.evaluate(elapsed => { fixture.wizardTranscript = { text: 'Testing Lectus.', language: 'pt', audio_ms: 3000, elapsed_ms: elapsed }; }, elapsed);
      await page.getByRole('button', { name: 'Test my microphone', exact: true }).click();
      await page.getByRole('button', { name: 'Show what I said', exact: true }).click();
      await page.getByText('Testing Lectus.', { exact: true }).waitFor();
      const offer = page.getByText('This computer is fast. Use the most accurate model (574 MB)?', { exact: true });
      assert.equal(await offer.count(), fast ? 1 : 0);
      if (fast) {
        await page.getByRole('button', { name: 'Use the most accurate model', exact: true }).click();
        await page.waitForFunction(() => fixture.wizardDownloads.includes('ggml-large-v3-turbo-q5_0.bin') && fixture.config.model_name === 'ggml-large-v3-turbo-q5_0.bin');
      }
    });
  }
  await checked('Wizard download failure explains the missing model and permits Continue', async page => {
    await openWizard(page);
    await page.evaluate(() => { fixture.wizardDownloadError = 'Fixture offline'; });
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page.getByRole('alert').filter({ hasText: 'Fixture offline' }).waitFor();
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page.getByText('The speech model is not downloaded yet. You can test later in Home.', { exact: true }).waitFor();
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page.getByRole('heading', { name: 'Your words', exact: true }).waitFor();
  });
  await checked('Leaving the wizard microphone step stops capture', async page => {
    await toMic(page);
    await page.getByRole('button', { name: 'Test my microphone', exact: true }).click();
    await page.waitForFunction(() => fixture.wizardRecording === true);
    const before = await page.evaluate(() => fixture.calls.filter(c => c === 'wizard_mic_stop').length);
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page.waitForFunction(before => fixture.calls.filter(c => c === 'wizard_mic_stop').length > before && !fixture.wizardRecording, before);
  });
  for (const toggle of [false, true]) {
    await checked(`Wizard shortcut ${toggle ? 'toggle' : 'hold'} test appends text without completing setup`, async page => {
      await page.evaluate(toggle => { fixture.config.trigger_mode = toggle ? 'toggle' : 'hold'; if (toggle) fixture.config.toggle_hotkey = 'RControl'; }, toggle);
      await toMic(page);
      await page.getByRole('button', { name: 'Continue', exact: true }).click();
      await page.getByRole('button', { name: /^(Continue|Skip)$/ }).click();
      await page.getByLabel('Try it', { exact: true }).focus();
      const key = 'ControlRight';
      await page.keyboard.down(key);
      await page.waitForFunction(() => fixture.wizardRecording === true);
      await page.keyboard.up(key);
      if (toggle) { assert.equal(await page.evaluate(() => fixture.wizardRecording), true); await page.keyboard.press(key); }
      await page.waitForFunction(() => document.querySelector('[aria-label="Try it"]')?.value === 'Testing Lectus.');
      assert.equal(await page.evaluate(() => fixture.calls.includes('complete_onboarding')), false);
    });
  }
}
