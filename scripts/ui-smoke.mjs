/** Browser fixture smoke tests, NOT native Tauri integration.
 * Run: node scripts/ui-smoke.mjs
 * Requires playwright or playwright-core resolvable, or PLAYWRIGHT_MODULE pointing
 * to an existing package directory. PLAYWRIGHT_CHROMIUM_EXECUTABLE is optional.
 * UI_SMOKE_API=1 runs Local API fixtures and writes separate API evidence.
 * UI_SMOKE_GUIDED=1 runs guided setup/help/translation/privacy fixtures.
 * UI_SMOKE_MODEL_LICENSE=1 runs synthetic Gemma agreement fixtures.
 * Starts and stops its own loopback-only Vite process; never invokes Rust.
 */
import { createRequire } from 'node:module';
import { spawn } from 'node:child_process';
import { mkdir, writeFile, readdir, readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import assert from 'node:assert/strict';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = fileURLToPath(new URL('..', import.meta.url));
const subscriptionMode = process.env.UI_SMOKE_SUBSCRIPTIONS === '1';
const apiMode = process.env.UI_SMOKE_API === '1';
const guidedMode = process.env.UI_SMOKE_GUIDED === '1';
const wizardMode = process.env.UI_SMOKE_WIZARD === '1';
const modelLicenseMode = process.env.UI_SMOKE_MODEL_LICENSE === '1';
const downloadMode = process.env.UI_SMOKE_DOWNLOADS === '1';
const output = path.join(root, process.env.UI_SMOKE_OUTPUT || (wizardMode ? 'docs/reviews/2026-11-13-wizard-smoke' : subscriptionMode ? 'docs/reviews/2026-10-22-subscription-ui-smoke' : downloadMode ? 'docs/reviews/2026-10-21-download-ui-smoke' : modelLicenseMode ? 'docs/reviews/2026-10-21-model-license-smoke' : guidedMode ? 'docs/reviews/2026-10-21-guided-smoke' : apiMode ? 'docs/reviews/2026-10-21-api-ui-smoke' : 'docs/reviews/2026-10-21-ui-smoke'));
async function fingerprintFrontend() {
  const files = [];
  async function walk(dir) {
    for (const entry of await readdir(path.join(root, dir), { withFileTypes: true })) {
      const relative = path.join(dir, entry.name);
      if (entry.isDirectory()) await walk(relative); else if (entry.isFile()) files.push(relative);
    }
  }
  await walk('src');
  files.push('index.html', 'package-lock.json');
  files.sort();
  const digest = createHash('sha256');
  for (const file of files) { digest.update(file.replaceAll('\\', '/')); digest.update('\0'); digest.update(await readFile(path.join(root, file))); digest.update('\0'); }
  return { sha256: digest.digest('hex'), file_count: files.length, scope: 'src/**, index.html, package-lock.json; sorted relative paths and exact bytes; no native/backend-source claim' };
}
const sourceBefore = await fingerprintFrontend();
const port = process.env.UI_SMOKE_PORT || '4179';
const url = `http://127.0.0.1:${port}`;
const server = spawn(process.execPath, [path.join(root, 'node_modules/vite/bin/vite.js'), '--host', '127.0.0.1', '--port', port, '--strictPort'], { cwd: root, windowsHide: true, stdio: 'pipe' });
let serverOutput = '';
server.stdout.on('data', data => { serverOutput += data; });
server.stderr.on('data', data => { serverOutput += data; });
const results = [];
let browser;

// Mirror only observed commands and event APIs. Unknown commands fail loudly.
function installFixture(windowLabel = 'main') {
  const clone = value => JSON.parse(JSON.stringify(value));
  const state = window.fixture = {
    calls: [], unknown: [], saveDelay: 0, failSaves: false, saving: 0, maxSaving: 0, saves: [], saveRequests: [], cloudError: '',
    config: { model_path: '', use_cloud: false, cloud_base_url: 'https://api.groq.com/openai/v1', cloud_api_key: '', cloud_transcription_model: 'whisper-large-v3-turbo', cloud_cleanup_model: 'openai/gpt-oss-120b', output_language: 'same', translation_engine: 'local', translation_model: 'Qwen3-4B-Q4_K_M.gguf', cloud_translation_model: 'openai/gpt-oss-120b', cloud_processing_acknowledged: false, claude_processing_acknowledged: false, chatgpt_processing_acknowledged: false, chatgpt_model: '', claude_model: '', hold_hotkey: 'RControl', toggle_hotkey: 'F13', pill_x: 0, pill_y: 0, language: 'auto', model_name: 'ggml-base.bin', trigger_mode: 'hold', dictionary_words: [], replacement_rules: [], ai_cleanup_enabled: true, ai_cleanup_engine: 'groq', ai_cleanup_tone: 'neutral', input_device: '', injection_mode: 'auto', vad_enabled: true, app_profiles: [], onboarding_completed: true, theme: 'light', mute_while_dictating: true, display_name: 'Alex' },
    history: [{ text: 'Review the launch notes tomorrow morning.', timestamp: 1792573200000, language: 'en' }, { text: 'Confirmar a reunião de equipa para sexta-feira.', timestamp: 1792580400000, language: 'pt' }],
    apiStatus: { enabled: false, running: false, busy: false, port: 3210, allowed_origins: [], error: null, calls: [] },
    connections: [
      { provider: 'chatgpt', state: 'disconnected', account_label: null, message: 'Sign in to use your ChatGPT plan.' },
      { provider: 'claude', state: 'disconnected', account_label: null, message: 'Sign in through official Claude Code.' },
    ], subscriptionActions: [], subscriptionTests: [], subscriptionError: '', subscriptionModelError: '',
    apiSaves: [], apiError: '', apiToken: 'fixture-token-not-a-secret', clipboardWrites: [],
    legal: { ready: false, accepted: false, terms_version: 'fixture-draft', privacy_version: 'fixture-draft', terms_hash: 'fixture-terms-hash', privacy_hash: 'fixture-privacy-hash', terms: '# Fixture Terms — synthetic publisher only', privacy: '# Fixture Privacy — synthetic facts only' },
    cleanupModel: { name: 'gemma-3-1b-it-Q4_K_M.gguf', downloaded: false, size_mb: 769, download: { downloading: false, progress: 0, error: null } },
    cleanupLicense: { model_id: 'gemma-3-1b', title: 'Synthetic Gemma fixture documents', version: 'fixture-v1', fingerprint: 'fixture-model-fingerprint-v1', accepted: false, terms: Array.from({ length: 30 }, (_, i) => 'Synthetic model term ' + (i + 1) + ': fixture text for offline reading and scrolling; no real agreement is accepted.').join('\n\n'), policy: 'Synthetic incorporated policy. Fixture-only restrictions. End of synthetic policy.' },
    modelAcceptRequests: [],
    translationModel: { name: 'Qwen3-4B-Q4_K_M.gguf', display_name: 'Qwen3 4B', size_mb: 2497, downloaded: false, download: { downloading: false, progress: 0, error: null } },
    models: [
      { name: 'ggml-base.bin', display_name: 'Base', size_mb: 142, multilingual: true, description: 'Fast recognition for everyday dictation.', downloaded: true, active: true, loading: false, download: { downloading: false, progress: 1, error: null } },
      { name: 'ggml-large-v3-turbo.bin', display_name: 'Large v3 Turbo', size_mb: 1620, multilingual: true, description: 'Higher accuracy with a larger memory footprint.', downloaded: false, active: false, loading: false, download: { downloading: false, progress: 0, error: null } },
    ],
  };
  if (new URLSearchParams(location.search).has('fixtureLegalRequired')) {
    state.legal.ready = true; state.legal.terms_version = 'fixture-1'; state.legal.privacy_version = 'fixture-1';
  }
  const callbacks = new Map();
  const listeners = new Map();
  let id = 0;
  state.emit = (event, payload) => { for (const handler of listeners.get(event) || []) callbacks.get(handler)?.({ event, payload, id: handler }); };
  window.__TAURI_OS_PLUGIN_INTERNALS__ = { platform: 'windows', family: 'windows', type: 'windows', version: 'fixture', eol: '\r\n' };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: (_event, key) => callbacks.delete(key) };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: windowLabel }, currentWebview: { label: windowLabel } },
    transformCallback: cb => { callbacks.set(++id, cb); return id; },
    unregisterCallback: key => callbacks.delete(key),
    invoke: async (command, args = {}) => {
      state.calls.push(command);
      if (command === 'plugin:event|listen') { const list = listeners.get(args.event) || []; list.push(args.handler); listeners.set(args.event, list); return args.handler; }
      if (command === 'plugin:event|unlisten') { listeners.set(args.event, (listeners.get(args.event) || []).filter(key => key !== args.eventId)); return; }
      if (command === 'plugin:event|emit') { state.emit(args.event, args.payload); return; }
      switch (command) {
        case 'get_ai_connections': state.connectionChecks ??= []; state.connectionChecks.push(clone(args)); return clone(state.connections);
        case 'start_ai_login': {
          state.subscriptionActions.push(clone({ command, ...args }));
          if (state.subscriptionError) throw new Error(state.subscriptionError);
          Object.assign(state.connections.find(c => c.provider === args.provider), { state: 'connecting', account_label: null, message: 'Finish sign-in in your browser.' });
          return;
        }
        case 'cancel_ai_login':
        case 'disconnect_ai': {
          state.subscriptionActions.push(clone({ command, ...args }));
          if (state.subscriptionError) throw new Error(state.subscriptionError);
          if (command === 'disconnect_ai' && state.deferSubscriptionDisconnect) await new Promise(resolve => { state.resolveSubscriptionDisconnect = resolve; });
          Object.assign(state.connections.find(c => c.provider === args.provider), { state: 'disconnected', account_label: null, message: 'Disconnected.' });
          return;
        }
        case 'list_subscription_models': {
          if (state.subscriptionModelError) throw new Error(state.subscriptionModelError);
          const models = args.provider === 'chatgpt' ? [{ id: 'fixture-text-model', name: 'Fixture text model' }, { id: 'fixture-second-model', name: 'Fixture second model' }] : [{ id: '', name: 'Claude default' }, { id: 'sonnet', name: 'Sonnet' }];
          if (state.deferSubscriptionModels) return new Promise(resolve => { state.resolveSubscriptionModels = () => resolve(models); });
          return models;
        }
        case 'test_subscription': {
          state.subscriptionTests.push(clone(args));
          if (state.subscriptionError) throw new Error(state.subscriptionError);
          if (state.deferSubscriptionTest) return new Promise(resolve => { state.resolveSubscriptionTest = () => resolve({ text: 'Lectus connection test.', elapsed_ms: 125 }); });
          return { text: 'Lectus connection test.', elapsed_ms: 125 };
        }
        case 'get_config': return clone(state.config);
        case 'hardware_profile': return { logical_cpus: 4, total_memory_bytes: 8 * 1024 ** 3, available_memory_bytes: 2 * 1024 ** 3 };
        case 'cancel_model_download': {
          const model = state.models.find(m => m.name === args.modelName)
            || (args.modelName === state.translationModel.name ? state.translationModel : state.cleanupModel);
          if (state.cancelError) throw new Error(state.cancelError);
          model.download = { ...model.download, downloading: false, paused: true, message: 'Download paused. Progress is saved.' };
          state.emit('model-download-state', { name: args.modelName, status: clone(model.download) });
          state.rejectPendingDownload?.(new Error('Download paused. Saved progress will resume when you continue.'));
          state.rejectPendingDownload = null;
          return;
        }
        case 'save_config': {
          state.saving++; state.maxSaving = Math.max(state.maxSaving, state.saving);
          const next = clone(args.newConfig);
          state.saveRequests.push(clone(next));
          try {
            await new Promise(resolve => setTimeout(resolve, state.saveDelay));
            if (state.failSaves) throw new Error('Fixture disk unavailable');
            if (state.config.cloud_base_url !== next.cloud_base_url) next.cloud_processing_acknowledged = false;
            next.onboarding_completed = state.config.onboarding_completed;
            state.config = next; state.saves.push(next); state.emit('theme-changed', next.theme);
          } finally { state.saving--; }
          return clone(next);
        }
        case 'get_api_status': return clone(state.apiStatus);
        case 'save_api_settings': {
          state.apiSaves.push(clone(args.settings));
          if (state.apiError) throw new Error(state.apiError);
          state.apiStatus = { ...state.apiStatus, ...clone(args.settings), running: args.settings.enabled && !state.apiListenerError, error: state.apiListenerError || null };
          state.emit('api-activity', clone(state.apiStatus));
          return clone(state.apiStatus);
        }
        case 'get_api_token': return state.apiToken;
        case 'regenerate_api_token': state.apiToken = 'fixture-rotated-token-not-a-secret'; return state.apiToken;
        case 'copy_to_clipboard': state.clipboardWrites.push(args.text); return;
        case 'get_legal_status': return clone(state.legal);
        case 'open_legal_notices': if (state.noticeError) throw new Error(state.noticeError); return;
        case 'accept_legal_terms':
          if (!state.legal.ready) throw new Error('Fixture draft terms cannot be accepted');
          if (state.legalAcceptError) throw new Error(state.legalAcceptError);
          if (args.termsVersion !== state.legal.terms_version || args.termsHash !== state.legal.terms_hash) throw new Error('Fixture terms changed');
          state.legal.accepted = true; return clone(state.legal);
        case 'license_floor': return { floor_usd: 20, tokens: [] };
        case 'microphone_status': return 'granted';
        case 'request_microphone_access': return;
        case 'complete_onboarding':
          if (state.finishError) throw new Error(state.finishError);
          if (!state.legal.ready || !state.legal.accepted) throw new Error('Fixture terms acceptance required');
          state.config.onboarding_completed = true; return;
        case 'translation_targets': return [{ code: 'same', label: 'Same as spoken' }, { code: 'en', label: 'English' }, { code: 'pt', label: 'Português' }, { code: 'zh-Hans', label: '简体中文 · Simplified Chinese' }, { code: 'zh-Hant', label: '繁體中文 · Traditional Chinese' }, { code: 'yue', label: '粵語 · Cantonese (Traditional)' }];
        case 'get_translation_model_status': return clone(state.translationModel);
        case 'download_translation_model': if (state.translationError) throw new Error(state.translationError); state.translationModel.downloaded = true; state.emit('model-ready', state.translationModel.name); return;
        case 'get_history': return clone(state.history);
        case 'clear_history': state.history = []; return;
        case 'get_history_stats': return { dictations_7d: 12, words_7d: 421, dictations_total: state.history.length, words_total: 421, streak_days: 3 };
        case 'remote_get_settings': return { mode: 'local', receiver_code: '', peer_code: '', status: 'This computer', error: false };
        case 'engine_ready': case 'check_claude_cli': return true;
        case 'license_status': return { kind: 'active', pubkey: 'FixtureWallet', balance: 100, usd: 50 };
        case 'os_display_name': return 'Alex';
        case 'plugin:app|version': return '0.7.1';
        case 'plugin:window|is_maximized': case 'plugin:autostart|is_enabled': return false;
        case 'plugin:autostart|enable': case 'plugin:autostart|disable': return;
        case 'take_whats_new_notes': case 'last_update_error': case 'check_for_updates': return null;
        case 'list_input_devices': return ['Fixture microphone'];
        case 'get_models_status': return clone(state.models);
        case 'get_cleanup_model_status': return clone(state.cleanupModel);
        case 'get_cleanup_model_license': if (state.modelLicenseLoadDelay) await new Promise(resolve => setTimeout(resolve, state.modelLicenseLoadDelay)); if (state.modelLicenseLoadError) throw new Error(state.modelLicenseLoadError); return clone(state.cleanupLicense);
        case 'accept_cleanup_model_license': {
          state.modelAcceptRequests.push(clone(args));
          if (state.modelAcceptError) throw new Error(state.modelAcceptError);
          if (args.fingerprint !== state.cleanupLicense.fingerprint) throw new Error('Fixture model fingerprint changed; reload terms');
          if (state.deferModelAccept) return new Promise(resolve => { state.resolveModelAccept = () => resolve({ ...clone(state.cleanupLicense), accepted: true }); });
          state.cleanupLicense.accepted = true; state.emit('cleanup-license-changed', clone(state.cleanupLicense)); return clone(state.cleanupLicense);
        }
        case 'revoke_cleanup_model_license':
          if (state.modelRevokeError) throw new Error(state.modelRevokeError);
          state.cleanupLicense.accepted = false; state.emit('cleanup-license-changed', clone(state.cleanupLicense)); return clone(state.cleanupLicense);
        case 'download_cleanup_model':
          if (!state.cleanupLicense.accepted) throw new Error('Fixture model terms required');
          if (state.cleanupDownloadError) throw new Error(state.cleanupDownloadError);
          state.cleanupModel.downloaded = true; return;
        case 'get_foreground_app': return new Promise(resolve => { state.resolveForeground = resolve; });
        case 'get_cloud_models': if (state.cloudDelay) await new Promise(resolve => setTimeout(resolve, state.cloudDelay)); if (state.cloudError) throw new Error(state.cloudError); return ['whisper-large-v3', 'whisper-large-v3-turbo', 'openai/gpt-oss-120b'];
        case 'download_model':
          if (state.wizard) {
            state.wizardDownloads.push(args.modelName);
            if (state.wizardDownloadError) throw new Error(state.wizardDownloadError);
            const model = state.models.find(m => m.name === args.modelName);
            model.downloaded = true;
            model.download = { downloading: false, progress: 1, error: null };
            state.emit('model-download-state', { name: model.name, status: clone(model.download) });
            return;
          }
          state.models[1].download = { downloading: true, progress: 0.42, error: null }; state.emit('model-download-state', null);
          if (state.deferModelDownload) return new Promise((_, reject) => { state.rejectPendingDownload = reject; });
          return;
        case 'select_model':
          state.config.model_name = args.modelName;
          for (const model of state.models) model.active = model.name === args.modelName;
          state.emit('model-active', args.modelName); return;
        default: state.unknown.push(command); throw new Error(`Unmocked command: ${command}`);
      }
    },
  };
}

async function pageFixture(viewport = { width: 1100, height: 800 }, windowLabel = 'main') {
  const page = await browser.newPage({ viewport, colorScheme: 'light' });
  page.setDefaultTimeout(8000);
  page.errors = [];
  page.externalRequests = [];
  await page.route('**/*', route => { if (route.request().url().startsWith(url + '/')) return route.continue(); page.externalRequests.push(route.request().url()); return route.abort(); });
  page.on('pageerror', error => page.errors.push(error.message));
  await page.addInitScript(installFixture, windowLabel);
  await page.goto(url, { timeout: 30000 });
  await (windowLabel === 'pill' ? page.locator('.pill-root') : page.getByRole('heading', { name: 'Welcome back, Alex' })).waitFor({ timeout: 20000 }).catch(async error => { console.error(await page.locator('body').innerText(), page.errors); throw error; });
  return page;
}
async function checked(name, test, windowLabel = 'main') {
  const page = await pageFixture(undefined, windowLabel);
  try {
    await test(page);
    assert.deepEqual(page.errors, [], 'browser runtime errors');
    assert.deepEqual(page.externalRequests, [], 'unexpected external browser requests');
    assert.deepEqual(await page.evaluate(() => fixture.unknown), [], 'unmocked commands');
    results.push({ name, passed: true }); console.log('PASS', name);
  } catch (error) {
    results.push({ name, passed: false, error: error.stack }); console.log('FAIL', name, error.message);
    await page.screenshot({ animations: 'disabled', path: path.join(output, `failed-${results.length}.png`) });
  } finally { await page.close(); }
}
const settings = page => page.getByRole('button', { name: 'Settings', exact: true }).click();
try {
  await mkdir(output, { recursive: true });
  for (let attempt = 0; attempt < 100; attempt++) {
    if (server.exitCode !== null) throw new Error(serverOutput);
    try { if ((await fetch(url)).ok) break; } catch { /* wait for owned Vite */ }
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  browser = await chromium.launch({ headless: true, ...(process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE ? { executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE } : {}) });
  if (wizardMode) await (await import('./wizard-smoke.mjs')).runWizardSmoke({ checked, pageFixture, output, results });
  if (subscriptionMode) {
    const aiSettings = async page => { await settings(page); await page.getByRole('button', { name: 'AI cleanup', exact: true }).click(); };
    const connectFixture = async (page, provider = 'chatgpt') => {
      await page.evaluate(provider => { Object.assign(fixture.connections.find(c => c.provider === provider), { state: 'connected', account_label: 'Fixture account', message: 'Subscription authenticated.' }); }, provider);
    };
    await checked('Login and cancellation never enable processing or API billing', async page => {
      await aiSettings(page);
      await page.getByRole('checkbox', { name: /Enable AI cleanup/ }).uncheck();
      await page.waitForFunction(() => fixture.config.ai_cleanup_enabled === false);
      await page.getByRole('button', { name: 'Sign in to ChatGPT', exact: true }).click();
      await page.getByRole('button', { name: 'Cancel ChatGPT sign-in', exact: true }).click();
      await page.getByRole('button', { name: 'Sign in to ChatGPT', exact: true }).waitFor();
      assert.deepEqual(await page.evaluate(() => fixture.subscriptionActions.map(a => a.command)), ['start_ai_login', 'cancel_ai_login']);
      assert.equal(await page.evaluate(() => fixture.config.ai_cleanup_enabled), false);
      assert.equal(await page.evaluate(() => fixture.config.chatgpt_processing_acknowledged), false);
      assert.equal(await page.evaluate(() => fixture.config.use_cloud), false);
      assert.equal(await page.evaluate(() => fixture.config.cloud_api_key), '');
    });
    await checked('Authenticated account requires real test and model selection, not private transcript consent', async page => {
      await connectFixture(page); await aiSettings(page);
      const card = page.getByRole('group', { name: 'ChatGPT connection', exact: true });
      await card.getByText(/text generation not verified/).waitFor();
      assert(await page.getByRole('button', { name: 'Test ChatGPT', exact: true }).isDisabled());
      await page.locator('#chatgpt-subscription-model').selectOption('fixture-text-model');
      await page.getByRole('button', { name: 'Test ChatGPT', exact: true }).click();
      await card.getByText(/Synthetic text test passed/).waitFor();
      assert.deepEqual(await page.evaluate(() => fixture.subscriptionTests), [{ provider: 'chatgpt', model: 'fixture-text-model' }]);
      assert.equal(await page.evaluate(() => fixture.config.chatgpt_processing_acknowledged), false);
    });
    await checked('Cleanup and translation independently use subscriptions and preserve speech mode', async page => {
      await connectFixture(page); await aiSettings(page);
      await page.getByRole('radio', { name: /ChatGPT/ }).check();
      await page.getByRole('checkbox', { name: /Allow transcript text processing through ChatGPT/ }).check();
      await page.locator('#chatgpt-subscription-model').selectOption('fixture-text-model');
      await page.waitForFunction(() => fixture.config.ai_cleanup_engine === 'chatgpt' && fixture.config.chatgpt_model === 'fixture-text-model' && fixture.config.chatgpt_processing_acknowledged);
      await page.getByRole('button', { name: 'Dictation', exact: true }).click();
      await page.locator('#output-language').selectOption('en');
      await page.locator('#translation-engine').selectOption('claude');
      await page.waitForFunction(() => fixture.config.translation_engine === 'claude');
      assert.equal(await page.evaluate(() => fixture.config.ai_cleanup_engine), 'chatgpt');
      assert.equal(await page.evaluate(() => fixture.config.use_cloud), false);
      assert.equal(await page.evaluate(() => fixture.config.cloud_api_key), '');
    });
    await checked('Failed subscription test is actionable and never echoes provider details', async page => {
      await connectFixture(page, 'claude'); await aiSettings(page);
      await page.evaluate(() => { fixture.subscriptionError = 'PRIVATE_PROVIDER_BODY_TOKEN'; });
      await page.getByRole('button', { name: 'Test Claude', exact: true }).click();
      await page.getByRole('alert').filter({ hasText: 'Text test failed' }).waitFor();
      assert(!(await page.locator('body').innerText()).includes('PRIVATE_PROVIDER_BODY_TOKEN'));
      assert.equal(await page.getByText(/Synthetic text test passed/).count(), 0);
      assert.equal(await page.evaluate(() => fixture.config.cloud_api_key), '');
    });
    await checked('Disconnect discards a late test result', async page => {
      await connectFixture(page, 'claude'); await aiSettings(page);
      await page.evaluate(() => { fixture.deferSubscriptionTest = true; });
      await page.getByRole('button', { name: 'Test Claude', exact: true }).click();
      await page.waitForFunction(() => !!fixture.resolveSubscriptionTest);
      await page.getByRole('button', { name: 'Disconnect Claude', exact: true }).click();
      await page.getByRole('button', { name: 'Sign in to Claude', exact: true }).waitFor();
      await page.evaluate(() => fixture.resolveSubscriptionTest());
      await page.waitForTimeout(100);
      assert.equal(await page.getByText(/Synthetic text test passed/).count(), 0);
    });
    await checked('Changing model discards success from previous model', async page => {
      await connectFixture(page); await aiSettings(page);
      await page.locator('#chatgpt-subscription-model').selectOption('fixture-text-model');
      await page.evaluate(() => { fixture.deferSubscriptionTest = true; });
      await page.getByRole('button', { name: 'Test ChatGPT', exact: true }).click();
      await page.waitForFunction(() => !!fixture.resolveSubscriptionTest);
      await page.locator('#chatgpt-subscription-model').selectOption('fixture-second-model');
      await page.evaluate(() => fixture.resolveSubscriptionTest());
      await page.waitForTimeout(100);
      assert.equal(await page.getByText(/Synthetic text test passed/).count(), 0);
    });
    await checked('Preferences cannot invalidate an in-flight disconnect and Check refreshes status', async page => {
      await connectFixture(page); await aiSettings(page);
      await page.locator('#chatgpt-subscription-model').selectOption('fixture-text-model');
      await page.evaluate(() => { fixture.deferSubscriptionDisconnect = true; });
      await page.getByRole('button', { name: 'Disconnect ChatGPT', exact: true }).click();
      await page.waitForFunction(() => !!fixture.resolveSubscriptionDisconnect);
      await page.locator('#chatgpt-subscription-model').selectOption('fixture-second-model');
      await page.getByRole('checkbox', { name: /Allow transcript text processing through ChatGPT/ }).check();
      await page.evaluate(() => fixture.resolveSubscriptionDisconnect());
      await page.getByRole('button', { name: 'Sign in to ChatGPT', exact: true }).waitFor();
      assert.equal(await page.getByRole('button', { name: 'Test ChatGPT', exact: true }).isDisabled(), true);
      await page.getByRole('button', { name: 'Check ChatGPT connection', exact: true }).click();
      await page.waitForFunction(() => fixture.connectionChecks.some(args => args.refresh === true));
    });
    await checked('Catalog failure permits retry without losing saved selection', async page => {
      await connectFixture(page);
      await aiSettings(page);
      await page.locator('#chatgpt-subscription-model').selectOption('fixture-text-model');
      await page.waitForFunction(() => fixture.config.chatgpt_model === 'fixture-text-model');
      await page.evaluate(() => { fixture.subscriptionModelError = 'Catalog unavailable'; });
      await page.getByRole('button', { name: 'Refresh ChatGPT models', exact: true }).click();
      await page.getByRole('alert').filter({ hasText: 'Could not load available models' }).waitFor();
      assert.equal(await page.locator('#chatgpt-subscription-model').inputValue(), 'fixture-text-model');
      await page.evaluate(() => { fixture.subscriptionModelError = ''; });
      await page.getByRole('button', { name: 'Refresh ChatGPT models', exact: true }).click();
      await page.locator('#chatgpt-subscription-model').selectOption('fixture-text-model');
    });
    await checked('Cleanup fallback notice is visible and dismissible', async page => {
      await page.evaluate(() => fixture.emit('processing-notice', 'AI cleanup was unavailable. The original transcript was kept.'));
      await page.getByText('AI cleanup was unavailable. The original transcript was kept.', { exact: true }).waitFor();
      await page.getByRole('button', { name: 'Dismiss processing notice', exact: true }).click();
      assert.equal(await page.getByText('AI cleanup was unavailable. The original transcript was kept.', { exact: true }).count(), 0);
    });
    for (const width of [1100, 720]) for (const theme of ['light', 'dark']) {
      const page = await pageFixture({ width, height: width === 1100 ? 800 : 540 });
      await connectFixture(page);
      await page.evaluate(theme => { fixture.config.theme = theme; fixture.emit('theme-changed', theme); }, theme);
      await aiSettings(page);
      const card = page.getByRole('group', { name: 'ChatGPT connection', exact: true });
      await card.scrollIntoViewIfNeeded();
      await page.screenshot({ animations: 'disabled', path: path.join(output, 'subscription-' + theme + '-' + width + '.png') });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      assert.equal(await page.evaluate(() => document.querySelector('.modal-body').scrollWidth > document.querySelector('.modal-body').clientWidth + 1), false);
      assert.deepEqual(page.errors, []);
      await page.close();
    }
    results.push({ name: '4 subscription light/dark viewport captures and overflow checks', passed: true });
  }
  if (downloadMode) {
    await checked('Voluntary pause does not surface as a failed download', async page => {
      await page.evaluate(() => { fixture.deferModelDownload = true; });
      await settings(page); await page.getByRole('button', { name: 'Models', exact: true }).click();
      await page.getByRole('button', { name: 'Download', exact: true }).click();
      await page.getByRole('button', { name: 'Pause download', exact: true }).click();
      await page.waitForFunction(() => [...document.querySelectorAll('button')].some(button => button.textContent === 'Resume download' && !button.disabled));
      assert.equal(await page.getByRole('alert').filter({ hasText: 'Download paused' }).count(), 0);
    });
    await checked('Download pauses without losing progress, survives panel reopen and resumes', async page => {
      await settings(page); await page.getByRole('button', { name: 'Models', exact: true }).click();
      await page.getByRole('button', { name: 'Download', exact: true }).click();
      await page.getByRole('progressbar').waitFor();
      assert.equal(await page.getByRole('progressbar').getAttribute('aria-valuenow'), '42');
      await page.getByRole('button', { name: 'Pause download', exact: true }).click();
      await page.getByRole('button', { name: 'Resume download', exact: true }).waitFor();
      assert.equal(await page.getByRole('progressbar').getAttribute('aria-valuenow'), '42');
      assert.equal(await page.getByRole('button', { name: 'Use this model', exact: true }).count(), 0);
      await page.getByRole('button', { name: 'General', exact: true }).click();
      await page.getByRole('button', { name: 'Models', exact: true }).click();
      await page.getByRole('button', { name: 'Resume download', exact: true }).click();
      await page.getByRole('button', { name: 'Pause download', exact: true }).waitFor();
    });
    await checked('Pause failure remains actionable and does not claim saved progress', async page => {
      await settings(page); await page.getByRole('button', { name: 'Models', exact: true }).click();
      await page.getByRole('button', { name: 'Download', exact: true }).click();
      await page.evaluate(() => { fixture.cancelError = 'Fixture cannot pause'; });
      await page.getByRole('button', { name: 'Pause download', exact: true }).click();
      await page.getByRole('alert').filter({ hasText: 'Fixture cannot pause' }).waitFor();
      assert.equal(await page.getByRole('button', { name: 'Resume download', exact: true }).count(), 0);
      await page.evaluate(() => { fixture.cancelError = ''; });
      await page.getByRole('button', { name: 'Pause download', exact: true }).click();
      await page.getByRole('button', { name: 'Resume download', exact: true }).waitFor();
    });
    await checked('Paused download keeps its actionable disk error across panel reopening', async page => {
      await page.evaluate(() => { fixture.models[1].download = { downloading: false, paused: true, progress: 0.42, downloaded_bytes: 42000000, total_bytes: 100000000, error: 'Fixture disk full. Free space and resume.', message: 'Saved progress is kept.' }; });
      await settings(page); await page.getByRole('button', { name: 'Models', exact: true }).click();
      await page.getByRole('alert').filter({ hasText: 'Fixture disk full' }).waitFor();
      await page.getByRole('button', { name: 'General', exact: true }).click();
      await page.getByRole('button', { name: 'Models', exact: true }).click();
      await page.getByRole('alert').filter({ hasText: 'Fixture disk full' }).waitFor();
      await page.getByRole('button', { name: 'Resume download', exact: true }).waitFor();
      assert.equal(await page.getByRole('progressbar').getAttribute('aria-valuenow'), '42');
    });
    await checked('Translation can pause and resume without marking a partial model ready', async page => {
      await page.evaluate(() => {
        fixture.translationModel.download = { downloading: true, progress: 0.61, downloaded_bytes: 61000000, total_bytes: 100000000, error: null };
      });
      await settings(page); await page.getByRole('button', { name: 'Dictation', exact: true }).click();
      await page.getByLabel('Output language', { exact: true }).selectOption('pt');
      await page.getByRole('button', { name: 'Pause download', exact: true }).click();
      await page.getByRole('button', { name: 'Resume translation download', exact: true }).waitFor();
      assert.equal(await page.evaluate(() => fixture.translationModel.downloaded), false);
      assert.equal(await page.getByRole('progressbar').getAttribute('aria-valuenow'), '61');
      await page.getByRole('button', { name: 'Resume translation download', exact: true }).click();
      await page.getByText(/Qwen3 4B.*Downloaded/).waitFor();
    });
    await checked('Cleanup pause preserves the separate model agreement gate', async page => {
      await page.evaluate(() => {
        fixture.cleanupLicense.accepted = true;
        fixture.cleanupModel.download = { downloading: true, progress: 0.23, downloaded_bytes: 23000000, total_bytes: 100000000, error: null };
      });
      await settings(page); await page.getByRole('button', { name: 'AI cleanup', exact: true }).click();
      await page.getByRole('radio', { name: /Local \(offline\)/ }).check();
      await page.getByRole('button', { name: 'Pause download', exact: true }).click();
      await page.getByRole('button', { name: 'Resume cleanup download', exact: true }).waitFor();
      assert.equal(await page.evaluate(() => fixture.cleanupModel.downloaded), false);
      await page.evaluate(() => { fixture.cleanupLicense.accepted = false; fixture.emit('cleanup-license-changed', structuredClone(fixture.cleanupLicense)); });
      await page.getByRole('checkbox', { name: 'I accept the Gemma Terms and incorporated use restrictions' }).waitFor();
      assert(await page.getByRole('button', { name: 'Resume cleanup download', exact: true }).isDisabled());
    });
    for (const width of [1100, 720]) for (const theme of ['light', 'dark']) {
      const page = await pageFixture({ width, height: width === 1100 ? 800 : 540 });
      await page.evaluate(theme => {
        fixture.config.theme = theme; fixture.emit('theme-changed', theme);
        fixture.models[1].download = { downloading: false, paused: true, progress: 0.42, downloaded_bytes: 42000000, total_bytes: 100000000, error: 'Fixture disk full. Free space and resume.', message: 'Downloaded progress is kept.' };
      }, theme);
      await settings(page); await page.getByRole('button', { name: 'Models', exact: true }).click();
      await page.getByRole('button', { name: 'Resume download', exact: true }).scrollIntoViewIfNeeded();
      await page.screenshot({ animations: 'disabled', path: path.join(output, `downloads-${theme}-${width}.png`) });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      assert.equal(await page.evaluate(() => document.querySelector('.modal-body').scrollWidth > document.querySelector('.modal-body').clientWidth + 1), false);
      await page.close();
    }
    results.push({ name: '4 download recovery light/dark viewport captures and overflow checks', passed: true });
  }
  if (!wizardMode && !apiMode && !guidedMode && !modelLicenseMode && !downloadMode && !subscriptionMode) {
  await checked('Keyboard switches, theme, typing focus and modal focus return', async page => {
    await settings(page);
    const name = page.getByRole('textbox', { name: 'Your name', exact: true });
    await name.fill(''); await name.pressSequentially('João da Silva', { delay: 75 });
    await page.waitForTimeout(1100);
    assert.equal(await name.inputValue(), 'João da Silva'); assert(await name.evaluate(el => el === document.activeElement));
    const vad = page.getByRole('switch', { name: 'Voice activity detection' });
    await vad.focus(); await page.keyboard.press('Space'); assert.equal(await vad.getAttribute('aria-checked'), 'false');
    await page.keyboard.press('Enter'); assert.equal(await vad.getAttribute('aria-checked'), 'true');
    await page.getByRole('button', { name: 'Dark', exact: true }).click();
    await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
    assert.equal(await page.getByRole('button', { name: 'Dark', exact: true }).getAttribute('aria-pressed'), 'true');
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => document.activeElement?.hasAttribute('data-settings-trigger'));
  });
  await checked('Autosave serializes overlapping revisions', async page => {
    await settings(page);
    await page.evaluate(() => { fixture.saveDelay = 1100; });
    const urlField = page.getByRole('textbox', { name: 'Provider URL' });
    await urlField.fill('https://first.example/v1');
    await page.waitForFunction(() => fixture.saving === 1);
    await urlField.fill('https://second.example/v1');
    await page.waitForFunction(() => fixture.saves.length === 2);
    assert.equal(await page.evaluate(() => fixture.maxSaving), 1);
    assert.equal(await page.evaluate(() => fixture.config.cloud_base_url), 'https://second.example/v1');
  });
  await checked('Edit during focus flush survives a stale backend response', async page => {
    await settings(page);
    await page.evaluate(() => { fixture.saveDelay = 1100; });
    const field = page.getByRole('textbox', { name: 'Provider URL' });
    await field.fill('https://flush-b.example/v1');
    await page.evaluate(() => fixture.emit('tauri://focus', null));
    await page.waitForFunction(() => fixture.saving === 1);
    await field.fill('https://latest-c.example/v1');
    await page.waitForFunction(() => fixture.saves.length >= 1);
    assert.equal(await field.inputValue(), 'https://latest-c.example/v1');
    await page.waitForFunction(() => fixture.config.cloud_base_url === 'https://latest-c.example/v1');
    assert.equal(await field.inputValue(), 'https://latest-c.example/v1');
  });
  await checked('Failed save plus focus retains draft and retries successfully', async page => {
    await settings(page);
    await page.evaluate(() => { fixture.failSaves = true; });
    const urlField = page.getByRole('textbox', { name: 'Provider URL' });
    await urlField.fill('https://unsaved.example/v1');
    await page.getByText(/Changes could not be saved/).waitFor();
    await page.evaluate(() => fixture.emit('tauri://focus', null));
    await page.waitForTimeout(100);
    assert.equal(await urlField.inputValue(), 'https://unsaved.example/v1');
    assert.equal(await page.evaluate(() => fixture.config.cloud_base_url), 'https://api.groq.com/openai/v1');
    await page.evaluate(() => { fixture.failSaves = false; fixture.emit('tauri://focus', null); });
    await page.waitForFunction(() => fixture.config.cloud_base_url === 'https://unsaved.example/v1');
  });
  await checked('History deletion cancel, focus return, confirmed clear', async page => {
    await page.getByRole('button', { name: 'Dictations', exact: true }).click();
    const clear = page.getByRole('button', { name: 'Clear history', exact: true });
    await clear.click();
    const cancel = page.getByRole('button', { name: 'Cancel', exact: true });
    assert(await cancel.evaluate(el => el === document.activeElement));
    assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'clear_history').length), 0);
    await cancel.click(); await page.waitForFunction(() => document.activeElement?.textContent === 'Clear history');
    await clear.click(); await page.getByRole('button', { name: 'Delete all dictations' }).click();
    await page.getByText('History cleared.', { exact: true }).waitFor();
    assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'clear_history').length), 1);
  });
  await checked('Models retained progress, errors and retry affordance', async page => {
    await settings(page); await page.getByRole('button', { name: 'Models', exact: true }).click();
    await page.getByRole('button', { name: 'Download', exact: true }).click();
    assert.equal(await page.getByRole('progressbar').getAttribute('aria-valuenow'), '42');
    await page.getByRole('button', { name: 'General', exact: true }).click();
    await page.getByRole('button', { name: 'Models', exact: true }).click();
    await page.getByRole('progressbar').waitFor();
    await page.evaluate(() => { fixture.models[1].download = { downloading: false, progress: 0, error: 'Fixture network interrupted' }; fixture.emit('model-download-state', null); });
    await page.getByRole('alert').filter({ hasText: 'Fixture network interrupted' }).waitFor();
    await page.getByRole('button', { name: 'Retry download' }).click();
    await page.getByRole('progressbar').waitFor();
  });
  await checked('Cloud metadata check, missing IDs and provider failure', async page => {
    await settings(page);
    const check = page.getByRole('button', { name: 'Check connection', exact: true });
    assert(await check.isDisabled());
    await page.getByLabel('API key', { exact: false }).fill('fixture-only-not-a-secret');
    await page.getByRole('switch', { name: 'Use cloud transcription' }).click();
    await check.click(); await page.getByText(/inference access still needs a real dictation test/).waitFor();
    await page.getByLabel('Speech model').fill('unknown-fixture-model');
    await check.click(); await page.getByRole('alert').filter({ hasText: /these models are not listed/ }).waitFor();
    await page.evaluate(() => { fixture.cloudError = 'Fixture provider refused credentials'; });
    await check.click(); await page.getByRole('alert').filter({ hasText: /provider refused credentials/ }).waitFor();
  });
  await checked('Cloud errors redact an echoed key', async page => {
    await settings(page);
    await page.getByLabel('API key', { exact: false }).fill('fixture-only-not-a-secret');
    await page.evaluate(() => { fixture.cloudError = 'Provider echoed fixture-only-not-a-secret'; });
    await page.getByRole('button', { name: 'Check connection', exact: true }).click();
    const alert = page.getByRole('alert').filter({ hasText: 'Provider echoed' });
    await alert.waitFor();
    assert((await alert.textContent()).includes('[hidden]'));
    assert(!(await alert.textContent()).includes('fixture-only-not-a-secret'));
  });
  await checked('Cloud results for an edited credential and URL are discarded', async page => {
    await settings(page);
    await page.getByLabel('API key', { exact: false }).fill('fixture-only-not-a-secret');
    await page.evaluate(() => { fixture.cloudDelay = 800; fixture.cloudError = 'Late credential failure'; });
    await page.getByRole('button', { name: 'Check connection', exact: true }).click();
    await page.getByLabel('API key', { exact: false }).fill('replacement-fixture-key');
    await page.getByLabel('Provider URL').fill('https://replacement.example/v1');
    await page.waitForTimeout(900);
    assert(await page.getByRole('button', { name: 'Check connection', exact: true }).isEnabled());
    assert.equal(await page.getByRole('alert').filter({ hasText: 'Late credential failure' }).count(), 0);
    assert.equal(await page.getByText(/Provider reachable/).count(), 0);
  });
  await checked('Late app detection cannot overwrite a manually edited profile', async page => {
    await settings(page);
    await page.getByRole('button', { name: 'AI cleanup', exact: true }).click();
    await page.getByRole('button', { name: '+ Add app profile', exact: true }).click();
    const name = page.getByRole('textbox', { name: 'App name contains', exact: true });
    await name.fill('original');
    await page.getByRole('button', { name: 'Detect app for profile 1', exact: true }).click();
    await page.waitForFunction(() => typeof fixture.resolveForeground === 'function');
    await name.fill('manual-edit');
    await page.evaluate(() => fixture.resolveForeground('stale-detected.exe'));
    await page.waitForTimeout(100);
    assert.equal(await name.inputValue(), 'manual-edit');
    await page.waitForFunction(() => fixture.config.app_profiles[0]?.app_match === 'manual-edit');
    assert.equal(await name.inputValue(), 'manual-edit');
  });
  await checked('Cloud check ignores a stale result after cloud feature changes', async page => {
    await settings(page);
    await page.getByLabel('API key', { exact: false }).fill('fixture-only-not-a-secret');
    await page.evaluate(() => { fixture.cloudDelay = 800; });
    await page.getByRole('button', { name: 'Check connection', exact: true }).click();
    await page.getByRole('switch', { name: 'Use cloud transcription' }).click();
    await page.getByRole('button', { name: 'Check connection', exact: true }).waitFor();
    assert.equal(await page.getByText(/Provider reachable/).count(), 0);
    assert.equal(await page.getByText(/Checking provider/).count(), 0);
    await page.getByRole('button', { name: 'Check connection', exact: true }).click();
    await page.getByText(/inference access still needs a real dictation test/).waitFor();
  });
  await checked('Named dictation and dictionary controls support keyboard edits', async page => {
    await settings(page);
    await page.getByRole('button', { name: 'Dictation', exact: true }).click();
    await page.getByLabel('Spoken language', { exact: true }).selectOption('pt');
    const toggle = page.getByRole('radiogroup', { name: 'Trigger mode', exact: true }).getByRole('radio', { name: /Tap to toggle/ });
    await toggle.focus(); await page.keyboard.press('Space'); assert(await toggle.isChecked());
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: 'Vocabulary', exact: true }).click();
    const word = page.getByRole('textbox', { name: 'Custom words', exact: true });
    await word.fill('Eclectus'); await word.press('Enter');
    await page.getByRole('button', { name: 'Remove Eclectus', exact: true }).waitFor();
    await page.getByRole('button', { name: '+ Add rule', exact: true }).click();
    await page.getByRole('textbox', { name: 'Rule 1: spoken text', exact: true }).fill('lecture');
    await page.getByRole('textbox', { name: 'Rule 1: replacement text', exact: true }).fill('Lectus');
    const matchCase = page.getByRole('checkbox', { name: 'Rule 1: match case', exact: true });
    await matchCase.focus(); await page.keyboard.press('Space'); assert(await matchCase.isChecked());
    await page.waitForFunction(() => fixture.config.replacement_rules[0]?.case_sensitive === true);
    assert.deepEqual(await page.evaluate(() => ({ language: fixture.config.language, mode: fixture.config.trigger_mode, words: fixture.config.dictionary_words, rule: fixture.config.replacement_rules[0] })), { language: 'pt', mode: 'toggle', words: ['Eclectus'], rule: { from: 'lecture', to: 'Lectus', case_sensitive: true } });
  });
  for (const width of [1100, 720]) for (const theme of ['light', 'dark']) {
    const page = await pageFixture({ width, height: width === 1100 ? 800 : 540 });
    await page.evaluate(async theme => { fixture.config.theme = theme; fixture.emit('tauri://focus', null); fixture.emit('theme-changed', theme); await new Promise(resolve => setTimeout(resolve, 50)); }, theme);
    const capture = async surface => {
      await page.screenshot({ animations: 'disabled', path: path.join(output, `${surface}-${theme}-${width}.png`) });
      const overflow = await page.evaluate(() => ({ page: document.documentElement.scrollWidth > innerWidth, panels: [...document.querySelectorAll('.modal-body, .surface-body')].some(el => el.scrollWidth > el.clientWidth + 1) }));
      assert.deepEqual(overflow, { page: false, panels: false }, `${surface} ${theme} ${width} overflow`);
    };
    await capture('home'); await settings(page); await capture('general');
    await page.getByRole('button', { name: 'Models', exact: true }).click();
    await page.getByText('Large v3 Turbo', { exact: true }).waitFor(); await capture('models');
    await page.getByRole('button', { name: 'AI cleanup', exact: true }).click(); await capture('ai');
    await page.close();
  }
  results.push({ name: '16 light/dark viewport screenshots; page and content horizontal overflow checks', passed: true });
  }
  if (modelLicenseMode) {
    const choice = page => page.getByRole('checkbox', { name: 'I accept the Gemma Terms and incorporated use restrictions', exact: true });
    const acceptButton = page => page.getByRole('button', { name: 'Accept model terms', exact: true });
    const downloadButton = page => page.getByRole('button', { name: 'Download model', exact: true });
    const openLocal = async page => {
      await settings(page); await page.getByRole('button', { name: 'AI cleanup', exact: true }).click();
      await page.getByRole('radio', { name: /Local \(offline\)/ }).check();
      await page.getByRole('heading', { name: 'Gemma model terms', exact: true }).waitFor();
    };
    const acceptModel = async page => {
      await choice(page).check(); await acceptButton(page).click();
      await page.getByRole('button', { name: 'Revoke model acceptance', exact: true }).waitFor();
    };
    await checked('Gemma selection does not imply acceptance or download; explicit action binds fingerprint', async page => {
      await openLocal(page); await choice(page).waitFor();
      assert(!(await choice(page).isChecked())); assert(await acceptButton(page).isDisabled()); assert(await downloadButton(page).isDisabled());
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => ['accept_cleanup_model_license', 'download_cleanup_model'].includes(c)).length), 0);
      await choice(page).check();
      assert.equal(await page.evaluate(() => fixture.modelAcceptRequests.length), 0);
      assert(await downloadButton(page).isDisabled());
      await acceptButton(page).click(); await page.getByRole('button', { name: 'Revoke model acceptance', exact: true }).waitFor();
      assert.deepEqual(await page.evaluate(() => fixture.modelAcceptRequests), [{ fingerprint: 'fixture-model-fingerprint-v1' }]);
      assert(await downloadButton(page).isEnabled());
      assert.equal(await page.getByText(/Current Gemma model terms (?:are )?accepted on this device\./).count(), 1);
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'download_cleanup_model').length), 0);
      assert.equal(await page.evaluate(() => fixture.config.cloud_processing_acknowledged), false);
    });
    await checked('Gemma full terms and policy are keyboard-readable without external requests', async page => {
      await openLocal(page);
      const termsSummary = page.getByText('Read Gemma Terms', { exact: true });
      await termsSummary.focus(); await page.keyboard.press('Enter');
      const terms = page.getByRole('region', { name: 'Read Gemma Terms', exact: true });
      assert.equal(await terms.locator('pre').innerText(), await page.evaluate(() => fixture.cleanupLicense.terms));
      await terms.focus(); await page.keyboard.press('End');
      await page.waitForFunction(() => { const el = [...document.querySelectorAll('[role=region]')].find(el => el.textContent.includes('Synthetic model term 30')); return el.scrollTop > 0; });
      assert((await terms.innerText()).includes('Synthetic model term 30'));
      const policySummary = page.getByText('Read incorporated use restrictions', { exact: true });
      await policySummary.focus(); await page.keyboard.press('Enter');
      assert.equal(await page.getByRole('region', { name: 'Read incorporated use restrictions', exact: true }).locator('pre').innerText(), await page.evaluate(() => fixture.cleanupLicense.policy));
      assert.equal(await page.evaluate(() => fixture.modelAcceptRequests.length), 0);
    });
    await checked('Existing Gemma model file still requires separate agreement', async page => {
      await page.evaluate(() => { fixture.cleanupModel.downloaded = true; });
      await openLocal(page);
      await page.getByText('Model downloaded. Accept the model terms before local cleanup can use it.', { exact: true }).waitFor();
      assert(!(await choice(page).isChecked())); assert.equal(await page.evaluate(() => fixture.cleanupLicense.accepted), false);
      await acceptModel(page);
      await page.getByText('Model downloaded. Local cleanup can use it.', { exact: true }).waitFor();
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'download_cleanup_model').length), 0);
    });
    await checked('Failed model-document load stays gated and explicit retry restores documents', async page => {
      await page.evaluate(() => { fixture.modelLicenseLoadError = 'Fixture unreadable documents'; });
      await openLocal(page);
      await page.getByRole('alert').filter({ hasText: 'Could not load the bundled Gemma terms' }).waitFor();
      assert(await downloadButton(page).isDisabled()); assert.equal(await acceptButton(page).count(), 0);
      await page.evaluate(() => { fixture.modelLicenseLoadError = ''; });
      await page.getByRole('button', { name: 'Retry model terms', exact: true }).click(); await choice(page).waitFor();
      assert(!(await choice(page).isChecked())); assert(await downloadButton(page).isDisabled());
    });
    await checked('Stale model fingerprint is rejected; reload requires a fresh unchecked choice', async page => {
      await openLocal(page); await choice(page).check();
      await page.evaluate(() => { fixture.cleanupLicense.fingerprint = 'fixture-model-fingerprint-v2'; fixture.cleanupLicense.version = 'fixture-v2'; });
      await acceptButton(page).click();
      await page.getByRole('alert').filter({ hasText: 'Fixture model fingerprint changed' }).waitFor();
      assert(await downloadButton(page).isDisabled());
      assert.equal(await page.evaluate(() => fixture.cleanupLicense.accepted), false);
      await page.evaluate(() => { fixture.modelLicenseLoadDelay = 350; });
      await page.getByRole('button', { name: 'Reload model terms', exact: true }).click();
      await page.getByText('Loading bundled model terms…', { exact: true }).waitFor();
      assert(await downloadButton(page).isDisabled());
      await choice(page).waitFor(); assert(!(await choice(page).isChecked()));
      await page.getByText('Synthetic Gemma fixture documents · version fixture-v2', { exact: true }).waitFor();
      await acceptModel(page);
      assert.equal(await page.evaluate(() => fixture.modelAcceptRequests.at(-1).fingerprint), 'fixture-model-fingerprint-v2');
    });
    await checked('Failed acceptance and download retries preserve separate model state', async page => {
      await openLocal(page);
      await page.evaluate(() => { fixture.modelAcceptError = 'Fixture acceptance disk failure'; });
      await choice(page).check(); await acceptButton(page).click();
      await page.getByRole('alert').filter({ hasText: 'Fixture acceptance disk failure' }).waitFor();
      assert(await downloadButton(page).isDisabled()); assert.equal(await page.evaluate(() => fixture.cleanupLicense.accepted), false);
      await page.evaluate(() => { fixture.modelAcceptError = ''; fixture.cleanupDownloadError = 'Fixture download failed'; });
      await acceptButton(page).click(); await page.getByRole('button', { name: 'Revoke model acceptance', exact: true }).waitFor();
      await downloadButton(page).click(); await page.getByRole('alert').filter({ hasText: 'Fixture download failed' }).waitFor();
      assert.equal(await page.evaluate(() => fixture.cleanupLicense.accepted), true);
      await page.evaluate(() => { fixture.cleanupDownloadError = ''; });
      await downloadButton(page).click(); await page.getByText('Model downloaded. Local cleanup can use it.', { exact: true }).waitFor();
    });
    await checked('Revocation failure preserves acceptance; retry blocks Gemma without switching cloud', async page => {
      await openLocal(page); await acceptModel(page);
      await page.evaluate(() => { fixture.modelRevokeError = 'Fixture revoke disk failure'; });
      await page.getByRole('button', { name: 'Revoke model acceptance', exact: true }).click();
      await page.getByRole('alert').filter({ hasText: 'Fixture revoke disk failure' }).waitFor();
      assert.equal(await page.evaluate(() => fixture.cleanupLicense.accepted), true); assert(await downloadButton(page).isEnabled());
      await page.evaluate(() => { fixture.modelRevokeError = ''; });
      await page.getByRole('button', { name: 'Revoke model acceptance', exact: true }).click();
      await choice(page).waitFor(); assert(!(await choice(page).isChecked())); assert(await downloadButton(page).isDisabled());
      assert(await page.getByRole('radio', { name: /Local \(offline\)/ }).isChecked());
      assert.equal(await page.evaluate(() => fixture.config.cloud_processing_acknowledged || fixture.config.claude_processing_acknowledged), false);
    });
    await checked('Model agreement events update downloaded-file gate and reset an outstanding choice', async page => {
      await page.evaluate(() => { fixture.cleanupModel.downloaded = true; });
      await openLocal(page); await choice(page).check();
      await page.evaluate(() => { fixture.cleanupLicense.fingerprint = 'fixture-event-v2'; fixture.emit('cleanup-license-changed', structuredClone(fixture.cleanupLicense)); });
      await page.waitForFunction(() => [...document.querySelectorAll('label')].some(label => label.textContent.includes('I accept the Gemma Terms and incorporated use restrictions') && label.querySelector('input')?.checked === false));
      assert(!(await choice(page).isChecked())); await acceptModel(page);
      await page.evaluate(() => { fixture.cleanupLicense.accepted = false; fixture.emit('cleanup-license-changed', structuredClone(fixture.cleanupLicense)); });
      await page.getByText('Model downloaded. Accept the model terms before local cleanup can use it.', { exact: true }).waitFor();
      assert(!(await choice(page).isChecked())); assert(await acceptButton(page).isDisabled());
    });
    await checked('Newer revocation event wins over delayed acceptance response', async page => {
      await openLocal(page); await page.evaluate(() => { fixture.deferModelAccept = true; });
      await choice(page).check(); await acceptButton(page).click();
      await page.waitForFunction(() => typeof fixture.resolveModelAccept === 'function');
      await page.evaluate(() => { fixture.cleanupLicense.accepted = false; fixture.emit('cleanup-license-changed', structuredClone(fixture.cleanupLicense)); fixture.resolveModelAccept(); });
      await page.getByText('Model terms are not accepted. Review the current documents before trying again.', { exact: true }).waitFor();
      assert(await downloadButton(page).isDisabled()); assert(!(await choice(page).isChecked()));
      assert.equal(await page.getByRole('button', { name: 'Revoke model acceptance', exact: true }).count(), 0);
    });
    for (const width of [1100, 720]) for (const theme of ['light', 'dark']) {
      const page = await pageFixture({ width, height: width === 1100 ? 800 : 540 });
      await page.evaluate(async theme => { fixture.config.theme = theme; fixture.emit('tauri://focus', null); fixture.emit('theme-changed', theme); await new Promise(resolve => setTimeout(resolve, 50)); }, theme);
      await openLocal(page); await choice(page).waitFor();
      for (const state of ['unaccepted', 'accepted']) {
        if (state === 'accepted') await acceptModel(page);
        await page.getByRole('heading', { name: 'Gemma model terms', exact: true }).evaluate(el => el.scrollIntoView({ block: 'start' }));
        await page.screenshot({ animations: 'disabled', path: path.join(output, `gemma-${state}-${theme}-${width}.png`) });
        assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth && [...document.querySelectorAll('.modal-body')].every(el => el.scrollWidth <= el.clientWidth + 1)), 'Gemma panel horizontal overflow');
      }
      assert.deepEqual(page.errors, []); assert.deepEqual(page.externalRequests, []);
      await page.close();
    }
    results.push({ name: '8 Gemma agreement-state screenshots across light-dark and both viewport sizes without horizontal overflow', passed: true });
  }
  if (guidedMode) {
    const openSetup = async (page, ready = false) => {
      await page.evaluate(ready => { fixture.config.onboarding_completed = false; fixture.legal.ready = ready; fixture.legal.accepted = false; fixture.legal.terms_version = ready ? 'fixture-1' : 'fixture-draft'; fixture.legal.privacy_version = ready ? 'fixture-1' : 'fixture-draft'; fixture.emit('tauri://focus', null); }, ready);
      await page.getByRole('button', { name: 'Continue', exact: true }).click();
      await page.getByRole('heading', { name: 'Privacy & terms', exact: true }).waitFor();
    };
    const acceptTerms = async (page, waitForConfirmation = true) => {
      await page.getByRole('checkbox', { name: 'I have read and agree to the Terms.', exact: true }).check();
      await page.getByRole('button', { name: 'Accept Terms', exact: true }).click();
      if (waitForConfirmation) await page.getByText('Current Terms accepted on this device.', { exact: true }).waitFor();
    };
    const reachReview = async page => {
      for (let step = 0; step < 4; step++) await page.getByRole('button', { name: /^(Continue|Skip)$/ }).click();
      await page.getByRole('heading', { name: '7 days free, starting now', exact: true }).waitFor();
    };
    await checked('Help stays inside Lectus and routes to relevant settings', async page => {
      await page.getByRole('button', { name: 'Help', exact: true }).click();
      await page.getByRole('heading', { name: 'Help & setup', exact: true }).waitFor();
      const disclosure = page.getByText('About translation', { exact: true });
      await disclosure.focus(); await page.keyboard.press('Enter');
      assert(await disclosure.evaluate(el => el.parentElement.open));
      for (const [button, heading] of [['Dictation settings', 'Dictation'], ['Open models', 'Models'], ['Cloud connection', 'General'], ['Local API settings', 'Local API']]) {
        await page.getByRole('button', { name: button, exact: true }).click();
        await page.getByRole('dialog', { name: 'Settings', exact: true }).getByRole('heading', { name: heading, exact: true }).waitFor();
        await page.keyboard.press('Escape');
      }
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'open_help').length), 0);
    });
    await checked('Contextual help starts collapsed and supports keyboard disclosure across panels', async page => {
      const toggleHelp = async title => {
        const summary = page.getByText(`About ${title}`, { exact: true });
        assert(!(await summary.evaluate(el => el.parentElement.open)), title);
        await summary.focus(); await page.keyboard.press('Enter');
        assert(await summary.evaluate(el => el.parentElement.open), title);
        await page.keyboard.press('Enter');
        assert(!(await summary.evaluate(el => el.parentElement.open)), title);
      };
      await settings(page);
      await toggleHelp('microphone & text insertion'); await toggleHelp('remote dictation');
      for (const [tab, topics] of [['Models', ['choosing a speech model']], ['AI cleanup', ['AI cleanup', 'app profiles']], ['Local API', ['the local API']]]) {
        await page.getByRole('button', { name: tab, exact: true }).click();
        for (const title of topics) await toggleHelp(title);
      }
      await page.keyboard.press('Escape');
      await page.getByRole('button', { name: 'Membership', exact: true }).click(); await toggleHelp('membership checks');
      await page.getByRole('button', { name: 'Vocabulary', exact: true }).click(); await toggleHelp('vocabulary & replacement rules');
    });
    await checked('Third-party license button invokes only its fixed command and surfaces errors', async page => {
      await page.getByRole('button', { name: 'Help', exact: true }).click();
      const openNotices = page.getByRole('button', { name: 'Open third-party licenses', exact: true });
      await openNotices.click();
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'open_legal_notices').length), 1);
      await page.evaluate(() => { fixture.noticeError = 'Fixture bundled notices unavailable'; });
      await openNotices.click();
      await page.getByRole('alert').filter({ hasText: 'Fixture bundled notices unavailable' }).waitFor();
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'open_legal_notices').length), 2);
    });
    await checked('Chinese output targets stay separate from spoken language and cloud speech', async page => {
      await settings(page); await page.getByRole('button', { name: 'Dictation', exact: true }).click();
      await page.getByLabel('Spoken language', { exact: true }).selectOption('pt');
      const outputLanguage = page.getByLabel('Output language', { exact: true });
      for (const target of ['zh-Hans', 'zh-Hant', 'yue']) { await outputLanguage.selectOption(target); assert.equal(await outputLanguage.inputValue(), target); }
      assert.equal(await page.getByLabel('Spoken language', { exact: true }).inputValue(), 'pt');
      await page.getByLabel('Translation engine', { exact: true }).selectOption('cloud');
      await page.getByLabel('Cloud translation model', { exact: true }).fill('fixture-translation-model');
      await page.getByText(/including when speech recognition is local/).waitFor();
      await page.waitForFunction(() => fixture.config.cloud_translation_model === 'fixture-translation-model');
      assert.equal(await page.evaluate(() => fixture.config.use_cloud), false);
      assert.equal(await page.evaluate(() => fixture.config.cloud_processing_acknowledged), false);
    });
    await checked('Local translation download surfaces failure and can retry', async page => {
      await settings(page); await page.getByRole('button', { name: 'Dictation', exact: true }).click();
      await page.getByLabel('Output language', { exact: true }).selectOption('zh-Hans');
      await page.evaluate(() => { fixture.translationError = 'Fixture download unavailable'; });
      await page.getByRole('button', { name: 'Download translation model', exact: true }).click();
      await page.getByRole('alert').filter({ hasText: 'Fixture download unavailable' }).waitFor();
      await page.evaluate(() => { fixture.translationError = ''; });
      await page.getByRole('button', { name: 'Download translation model', exact: true }).click();
      await page.getByText(/Qwen3 4B.*Downloaded/).waitFor();
    });
    await checked('Provider permission resets on URL edit; Claude permission is separate', async page => {
      await settings(page);
      const cloud = page.getByRole('checkbox', { name: /Allow the features I enable to send audio or text to this provider/ });
      assert(!(await cloud.isChecked())); await cloud.check();
      await page.getByLabel('Provider URL', { exact: false }).fill('https://new-provider.example/v1');
      assert(!(await cloud.isChecked()));
      await page.getByRole('button', { name: 'AI cleanup', exact: true }).click();
      await page.getByRole('radio', { name: /Claude subscription/ }).check();
      const cli = page.getByRole('checkbox', { name: /Allow transcript text processing through Claude/ });
      assert(!(await cli.isChecked())); await cli.check();
      await page.waitForFunction(() => fixture.config.claude_processing_acknowledged === true);
      assert.equal(await page.evaluate(() => fixture.config.cloud_processing_acknowledged), false);
    });
    await checked('Backend normalization clears a same-debounce provider acknowledgement', async page => {
      await settings(page);
      const permission = page.getByRole('checkbox', { name: /Allow the features I enable to send audio or text to this provider/ });
      await page.getByLabel('Provider URL', { exact: false }).fill('https://same-debounce.example/v1');
      await permission.check();
      assert(await permission.isChecked());
      await page.waitForFunction(() => fixture.saves.some(config => config.cloud_base_url === 'https://same-debounce.example/v1'));
      assert.equal(await page.evaluate(() => fixture.saveRequests.at(-1).cloud_processing_acknowledged), true);
      assert.equal(await page.evaluate(() => fixture.config.cloud_processing_acknowledged), false);
      await page.waitForFunction(() => [...document.querySelectorAll('input[type=checkbox]')].some(el => el.closest('label')?.textContent.includes('Allow the features I enable') && !el.checked));
      assert(!(await permission.isChecked()));
      await page.getByText('Saved', { exact: true }).waitFor();
      // A second, explicit choice after the provider URL is saved is retained.
      await permission.check();
      await page.waitForFunction(() => fixture.config.cloud_processing_acknowledged === true);
      assert(await permission.isChecked());
    });
    await checked('Existing users can read drafts; published unaccepted Terms gate only legal review', async page => {
      assert.equal(await page.evaluate(() => fixture.legal.ready), false);
      assert(await page.getByRole('heading', { name: 'Welcome back, Alex', exact: true }).isVisible());
      assert((await page.evaluate(() => fixture.calls.filter(c => c === 'get_legal_status').length)) >= 1);
      await page.getByRole('button', { name: 'Help', exact: true }).click();
      await page.getByRole('button', { name: 'Read privacy and terms', exact: true }).click();
      await page.getByText('Terms · fixture-draft', { exact: true }).click();
      await page.getByText('# Fixture Terms — synthetic publisher only', { exact: true }).waitFor();
      await page.getByText('Privacy Notice · fixture-draft', { exact: true }).click();
      await page.getByText('# Fixture Privacy — synthetic facts only', { exact: true }).waitFor();
      await page.goto(`${url}/?fixtureLegalRequired=1`);
      await page.getByRole('heading', { name: 'Review the current Terms', exact: true }).waitFor();
      assert.equal(await page.getByRole('button', { name: 'Get started', exact: true }).count(), 0);
      assert.equal(await page.getByRole('heading', { name: 'Welcome back, Alex', exact: true }).count(), 0);
      await acceptTerms(page, false);
      await page.getByRole('heading', { name: 'Welcome back, Alex', exact: true }).waitFor();
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'complete_onboarding' || c === 'request_microphone_access').length), 0);
      assert.equal(await page.evaluate(() => fixture.config.onboarding_completed), true);
      assert.equal(await page.evaluate(() => fixture.config.cloud_processing_acknowledged), false);
    });
    await checked('Draft legal documents block setup without presenting acceptance', async page => {
      await openSetup(page);
      await page.getByText(/These drafts are not a final agreement/).waitFor();
      assert(await page.getByRole('button', { name: 'Continue', exact: true }).isDisabled());
      assert.equal(await page.getByRole('button', { name: 'Accept Terms', exact: true }).count(), 0);
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => ['accept_legal_terms', 'request_microphone_access', 'complete_onboarding'].includes(c)).length), 0);
    });
    await checked('Explicit Terms and failed finish keep setup safe', async page => {
      await openSetup(page, true);
      assert(await page.getByRole('button', { name: 'Accept Terms', exact: true }).isDisabled());
      assert(await page.getByRole('button', { name: 'Continue', exact: true }).isDisabled());
      await acceptTerms(page);
      assert.equal(await page.evaluate(() => fixture.config.cloud_processing_acknowledged), false);
      assert.equal(await page.evaluate(() => fixture.config.claude_processing_acknowledged), false);
      await reachReview(page);
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'request_microphone_access').length), 0);
      await page.evaluate(() => { fixture.finishError = 'Fixture settings disk unavailable'; });
      await page.getByRole('button', { name: 'Finish setup', exact: true }).click();
      await page.getByRole('alert').filter({ hasText: 'Setup was not completed' }).waitFor();
      assert(await page.getByRole('heading', { name: '7 days free, starting now', exact: true }).isVisible());
      assert.equal(await page.evaluate(() => fixture.config.onboarding_completed), false);
      await page.evaluate(() => { fixture.finishError = ''; });
      await page.getByRole('button', { name: 'Finish setup', exact: true }).click();
      await page.getByRole('heading', { name: 'Welcome back, Alex', exact: true }).waitFor();
    });
    await checked('Translated history exposes original copy and failed-translation recovery', async page => {
      await page.evaluate(() => { fixture.history = [{ text: '你好 Alex，会议在周五。', timestamp: 1792573200000, language: 'zh-Hans', source_text: 'Olá Alex, a reunião é sexta-feira.', source_language: 'pt', translation_error: null }, { text: 'Keep the original sentence.', timestamp: 1792573201000, language: 'en', source_text: 'Keep the original sentence.', source_language: 'en', translation_error: 'Fixture translation failed' }]; });
      await page.getByRole('button', { name: 'Dictations', exact: true }).click();
      await page.getByText('Translation did not finish. This is the original text; it was not inserted.', { exact: true }).waitFor();
      await page.getByText('Original · pt', { exact: true }).click();
      await page.getByRole('button', { name: 'Copy original', exact: true }).click();
      await page.waitForFunction(() => fixture.clipboardWrites.includes('Olá Alex, a reunião é sexta-feira.'));
    });
    for (const width of [1100, 720]) for (const theme of ['light', 'dark']) {
      const page = await pageFixture({ width, height: width === 1100 ? 800 : 540 });
      await page.evaluate(async theme => { fixture.config.theme = theme; fixture.emit('tauri://focus', null); fixture.emit('theme-changed', theme); await new Promise(resolve => setTimeout(resolve, 50)); }, theme);
      const capture = async surface => {
        await page.screenshot({ animations: 'disabled', path: path.join(output, `${surface}-${theme}-${width}.png`) });
        assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `${surface} page overflow`);
        assert(await page.evaluate(() => [...document.querySelectorAll('.modal-body, .surface-body, .wizard-root')].every(el => el.scrollWidth <= el.clientWidth + 1)), `${surface} content overflow`);
      };
      await page.getByRole('button', { name: 'Help', exact: true }).click(); await capture('help');
      await page.getByRole('button', { name: 'Language & translation', exact: true }).click();
      await page.getByLabel('Output language', { exact: true }).selectOption('zh-Hans'); await capture('translation'); await page.keyboard.press('Escape');
      await page.waitForFunction(() => fixture.saving === 0 && fixture.config.output_language === 'zh-Hans');
      await openSetup(page, true); await capture('legal-setup'); await acceptTerms(page); await reachReview(page); await capture('review-setup');
      await page.close();
    }
    results.push({ name: '16 guided setup/help/translation light-dark viewport screenshots without horizontal overflow', passed: true });
  }
  if (apiMode) {
    const openApi = async page => { await settings(page); await page.getByRole('button', { name: 'Local API', exact: true }).click(); await page.getByRole('switch', { name: /Enable local API/ }).waitFor(); };
    const enableApi = async page => { await page.getByRole('switch', { name: /Enable local API/ }).click(); await page.getByRole('button', { name: 'Save API settings', exact: true }).click(); await page.getByText('Server running on this computer', { exact: true }).waitFor(); };
    await checked('API defaults off, edits stay draft, explicit save applies exact settings', async page => {
      await openApi(page);
      const enabled = page.getByRole('switch', { name: /Enable local API/ });
      assert.equal(await enabled.getAttribute('aria-checked'), 'false');
      assert(await page.getByRole('button', { name: 'Reveal token', exact: true }).isDisabled());
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'get_api_token').length), 0);
      await page.getByLabel('Port', { exact: false }).fill('3211');
      await page.getByLabel('Allowed browser origins', { exact: true }).fill('https://example.test\nhttp://localhost:3000\nhttps://example.test');
      await enabled.focus(); await page.keyboard.press('Space');
      await page.waitForTimeout(650);
      assert.equal(await page.evaluate(() => fixture.apiSaves.length), 0);
      await page.getByRole('button', { name: 'Save API settings', exact: true }).click();
      await page.getByText('Server running on this computer', { exact: true }).waitFor();
      assert.deepEqual(await page.evaluate(() => fixture.apiSaves), [{ enabled: true, port: 3211, allowed_origins: ['https://example.test', 'http://localhost:3000'] }]);
      await page.getByText('http://127.0.0.1:3211/v1/transcribe', { exact: true }).waitFor();
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'get_api_token').length), 0);
    });
    await checked('API rejects invalid drafts before IPC and preserves failed enable draft', async page => {
      await openApi(page);
      const portField = page.getByLabel('Port', { exact: false });
      const origins = page.getByLabel('Allowed browser origins', { exact: true });
      const save = page.getByRole('button', { name: 'Save API settings', exact: true });
      await portField.fill('0'); await save.click();
      await page.getByRole('alert').filter({ hasText: 'Enter a port from 1 to 65535.' }).waitFor();
      assert.equal(await page.evaluate(() => fixture.apiSaves.length), 0);
      await portField.fill('3212'); await origins.fill('https://example.test/path'); await save.click();
      await page.getByRole('alert').filter({ hasText: 'Enter an exact origin' }).waitFor();
      assert.equal(await page.evaluate(() => fixture.apiSaves.length), 0);
      await origins.fill('https://example.test');
      await page.getByRole('switch', { name: /Enable local API/ }).click();
      await page.evaluate(() => { fixture.apiError = 'Fixture port already in use'; });
      await save.click();
      await page.getByRole('alert').filter({ hasText: 'Fixture port already in use' }).waitFor();
      assert.equal(await portField.inputValue(), '3212');
      assert.equal(await page.getByRole('switch', { name: /Enable local API/ }).getAttribute('aria-checked'), 'true');
      assert.equal(await page.evaluate(() => fixture.apiStatus.enabled), false);
      await page.getByRole('button', { name: 'Discard changes', exact: true }).click();
      assert.equal(await portField.inputValue(), '3210');
      assert.equal(await page.getByRole('switch', { name: /Enable local API/ }).getAttribute('aria-checked'), 'false');
    });
    await checked('API saved enable can report listener failure without claiming readiness', async page => {
      await openApi(page);
      await page.evaluate(() => { fixture.apiListenerError = 'Fixture bind failure'; });
      await page.getByRole('switch', { name: /Enable local API/ }).click();
      await page.getByRole('button', { name: 'Save API settings', exact: true }).click();
      await page.getByRole('alert').filter({ hasText: 'Fixture bind failure' }).waitFor();
      await page.getByText('Needs attention', { exact: true }).waitFor();
      assert.equal(await page.getByText('Server running on this computer', { exact: true }).count(), 0);
      assert.equal(await page.evaluate(() => fixture.apiStatus.enabled), true);
      assert.equal(await page.evaluate(() => fixture.apiStatus.running), false);
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'get_api_token').length), 0);
      await page.evaluate(() => { fixture.apiListenerError = ''; });
      await page.getByRole('button', { name: 'Save API settings', exact: true }).click();
      await page.getByText('Server running on this computer', { exact: true }).waitFor();
    });
    await checked('API token reveal, hidden copy and confirmed rotation stay explicit', async page => {
      await openApi(page); await enableApi(page);
      const token = page.getByLabel('API token', { exact: true });
      assert.equal(await token.inputValue(), '');
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'get_api_token').length), 0);
      await page.getByRole('button', { name: 'Reveal token', exact: true }).click();
      await page.waitForFunction(() => document.querySelector('#api-token')?.value === 'fixture-token-not-a-secret');
      await page.getByRole('button', { name: 'Hide token', exact: true }).click();
      assert.equal(await token.inputValue(), '');
      await page.getByRole('button', { name: 'Copy token', exact: true }).click();
      await page.waitForFunction(() => fixture.clipboardWrites.length === 1);
      assert.equal(await token.inputValue(), '');
      assert.deepEqual(await page.evaluate(() => fixture.clipboardWrites), ['fixture-token-not-a-secret']);
      const regenerate = page.getByRole('button', { name: 'Regenerate token', exact: true });
      await regenerate.click();
      const cancel = page.getByRole('button', { name: 'Cancel', exact: true });
      assert(await cancel.evaluate(el => el === document.activeElement));
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'regenerate_api_token').length), 0);
      await cancel.click(); await page.waitForFunction(() => document.activeElement?.textContent === 'Regenerate token');
      await regenerate.click(); await page.getByRole('button', { name: 'Replace token', exact: true }).click();
      await page.getByText(/Token regenerated. The previous token no longer works/).waitFor();
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'regenerate_api_token').length), 1);
      assert.equal(await token.inputValue(), '');
      await page.getByRole('button', { name: 'Copy token', exact: true }).click();
      await page.waitForFunction(() => fixture.clipboardWrites.at(-1) === 'fixture-rotated-token-not-a-secret');
    });
    await checked('API activity updates busy, failures and last five calls without overwriting draft', async page => {
      await openApi(page); await enableApi(page);
      await page.getByLabel('Port', { exact: false }).fill('3219');
      await page.evaluate(() => {
        fixture.apiStatus.busy = true;
        fixture.emit('api-activity', structuredClone(fixture.apiStatus));
      });
      await page.getByText('Transcribing uploaded audio', { exact: true }).waitFor();
      assert.equal(await page.getByLabel('Port', { exact: false }).inputValue(), '3219');
      await page.evaluate(() => {
        fixture.apiStatus.busy = false;
        fixture.apiStatus.calls = Array.from({ length: 6 }, (_, index) => ({ timestamp_ms: 1792573200000 + index * 1000, origin: `https://client-${index}.example.test`, duration_ms: 1234, result_chars: index === 5 ? null : 80 + index, error: index === 5 ? 'Fixture unsupported audio' : null }));
        fixture.emit('api-activity', structuredClone(fixture.apiStatus));
      });
      await page.getByText('Fixture unsupported audio', { exact: true }).waitFor();
      assert.equal(await page.locator('.api-calls > li').count(), 5);
      assert.equal(await page.getByText('https://client-0.example.test', { exact: true }).count(), 0);
      assert.equal(await page.locator('.api-calls > li').first().getByText('https://client-5.example.test', { exact: true }).count(), 1);
      await page.evaluate(() => { fixture.apiStatus.running = false; fixture.apiStatus.error = 'Fixture listener unavailable'; fixture.emit('api-activity', structuredClone(fixture.apiStatus)); });
      await page.getByRole('alert').filter({ hasText: 'Fixture listener unavailable' }).waitFor();
      await page.getByText('Needs attention', { exact: true }).waitFor();
      assert.equal(await page.getByLabel('Port', { exact: false }).inputValue(), '3219');
    });
    await checked('Disabled API reports its in-flight request until completion', async page => {
      await openApi(page); await enableApi(page);
      await page.evaluate(() => { fixture.apiStatus = { ...fixture.apiStatus, enabled: false, running: false, busy: true }; fixture.emit('api-activity', structuredClone(fixture.apiStatus)); });
      await page.getByText('Finishing current API request', { exact: true }).waitFor();
      assert.equal(await page.getByRole('switch', { name: /Enable local API/ }).getAttribute('aria-checked'), 'false');
      assert.equal(await page.getByText('Off', { exact: true }).count(), 0);
      await page.evaluate(() => { fixture.apiStatus.busy = false; fixture.emit('api-activity', structuredClone(fixture.apiStatus)); });
      await page.getByText('Off', { exact: true }).waitFor();
      assert.equal(await page.getByText('Finishing current API request', { exact: true }).count(), 0);
    });
    await checked('Pill retains the disabled-busy API indicator until completion', async page => {
      await page.evaluate(() => { fixture.apiStatus = { ...fixture.apiStatus, enabled: false, running: false, busy: true }; fixture.emit('api-activity', structuredClone(fixture.apiStatus)); });
      const indicator = page.getByRole('status', { name: 'Local API is off. Finishing the current uploaded audio request.', exact: true });
      await indicator.waitFor();
      assert.match(await indicator.getAttribute('class'), /is-busy/);
      await page.evaluate(() => { fixture.apiStatus.busy = false; fixture.emit('api-activity', structuredClone(fixture.apiStatus)); });
      await page.locator('.pill-api-indicator').waitFor({ state: 'detached' });
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'get_api_token').length), 0);
    }, 'pill');
    for (const width of [1100, 720]) for (const theme of ['light', 'dark']) {
      const page = await pageFixture({ width, height: width === 1100 ? 800 : 540 });
      await page.evaluate(async theme => {
        fixture.config.theme = theme; fixture.emit('tauri://focus', null); fixture.emit('theme-changed', theme);
        fixture.apiStatus = { ...fixture.apiStatus, enabled: true, running: true, allowed_origins: ['https://example.test'], calls: [{ timestamp_ms: 1792573200000, origin: 'https://example.test', duration_ms: 1234, result_chars: 148, error: null }, { timestamp_ms: 1792573201000, origin: null, duration_ms: null, result_chars: null, error: 'Unsupported WAV format: use 16 kHz mono audio.' }] };
        await new Promise(resolve => setTimeout(resolve, 50));
      }, theme);
      await openApi(page);
      for (const region of ['settings', 'activity']) {
        if (region === 'activity') await page.getByRole('heading', { name: 'Recent calls', exact: true }).scrollIntoViewIfNeeded();
        await page.screenshot({ animations: 'disabled', path: path.join(output, `api-${region}-${theme}-${width}.png`) });
        const overflow = await page.evaluate(() => ({ page: document.documentElement.scrollWidth > innerWidth, panel: document.querySelector('.modal-body').scrollWidth > document.querySelector('.modal-body').clientWidth + 1 }));
        assert.deepEqual(overflow, { page: false, panel: false }, `API ${region} ${theme} ${width} overflow`);
      }
      assert.equal(await page.evaluate(() => fixture.calls.filter(c => c === 'get_api_token').length), 0);
      await page.close();
    }
    results.push({ name: '8 API light/dark viewport screenshots, hidden tokens and horizontal overflow checks', passed: true });
  }
} catch (error) { results.push({ name: 'Harness/screenshots', passed: false, error: error.stack }); }
finally {
  if (browser) await browser.close();
  server.kill();
  const sourceAfter = await fingerprintFrontend();
  if (sourceBefore.sha256 !== sourceAfter.sha256) results.push({ name: 'Frontend source remained unchanged during run', passed: false });
  await writeFile(path.join(output, 'results.json'), JSON.stringify({ sourceBefore, sourceAfter, timestamp: new Date().toISOString(), scope: 'Chromium browser fixture; all IPC mocked; no native integration or actual provider calls', results }, null, 2));
  console.log(JSON.stringify(results, null, 2));
  if (results.some(result => !result.passed)) process.exitCode = 1;
}
