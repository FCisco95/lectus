import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Config, LicenseStatus } from './types';
import { allowsDictation, formatHotkey } from './types';
import { HistoryList } from './HistoryList';

interface HistoryStats {
  dictations_7d: number;
  words_7d: number;
  dictations_total: number;
  words_total: number;
  streak_days: number;
}

interface HomePanelProps {
  config: Config;
  onOpenTab: (tab: 'vocabulary' | 'models' | 'membership') => void;
}

/** One line telling a locked or in-grace holder where they stand. Holders in
 *  good standing see nothing — the gate should be invisible when it passes. */
function membershipNotice(status: LicenseStatus | null): string | null {
  if (!status) return null;
  switch (status.kind) {
    case 'unlinked':
      return 'Link a wallet holding ORGANIC or Mycel to start dictating.';
    case 'grace':
      return `Your wallet is below the $20 floor. Dictation keeps working for ${status.days_left} more ${status.days_left === 1 ? 'day' : 'days'}.`;
    case 'locked':
      return 'Dictation is locked: this wallet no longer holds $20 of ORGANIC or Mycel.';
    default:
      return null;
  }
}

/** Greeting, week KPIs, then the dictation list so a prompt can be copied
 *  without leaving Home. The list scrolls; the KPIs stay put. */
export function HomePanel({ config, onOpenTab }: HomePanelProps) {
  const [stats, setStats] = useState<HistoryStats | null>(null);
  const [engineReady, setEngineReady] = useState<boolean | null>(null);
  const [engineError, setEngineError] = useState(false);
  const [licenseError, setLicenseError] = useState(false);
  const [statsError, setStatsError] = useState(false);
  const [retry, setRetry] = useState(0);
  const [destination, setDestination] = useState<'local' | 'send' | 'receive' | null>(null);
  const [license, setLicense] = useState<LicenseStatus | null>(null);
  const [osName, setOsName] = useState('');

  const loadStats = () => {
    // The streak's day boundaries are local midnight, which only the
    // webview knows; getTimezoneOffset() is minutes behind UTC.
    invoke<HistoryStats>('get_history_stats', { tzOffsetMin: new Date().getTimezoneOffset() })
      .then((value) => { setStats(value); setStatsError(false); })
      .catch(() => setStatsError(true));
  };

  useEffect(() => {
    loadStats();
    setEngineError(false);
    setLicenseError(false);
    invoke<boolean>('engine_ready')
      .then(setEngineReady)
      .catch(() => { setEngineReady(null); setEngineError(true); });
    invoke<LicenseStatus>('license_status').then(setLicense).catch(() => setLicenseError(true));
    invoke<string>('os_display_name').then(setOsName).catch(() => {});
    const loadDestination = () => invoke<{ mode: 'local' | 'send' | 'receive' }>('remote_get_settings')
      .then((settings) => setDestination(settings.mode)).catch(() => setDestination(null));
    loadDestination();
    const unRemote = listen('remote-delivery-status', loadDestination);
    const unLicense = listen<LicenseStatus>('license-changed', (e) => { setLicense(e.payload); setLicenseError(false); });
    const unAdded = listen('history-added', () => loadStats());
    const unReady = listen('model-active', () => { setEngineReady(true); setEngineError(false); });
    const unLoad = listen('model-loading', () => { setEngineReady(false); setEngineError(false); });
    const unFailed = listen('model-load-failed', () => { setEngineReady(false); setEngineError(true); });
    return () => {
      unLicense.then((f) => f());
      unAdded.then((f) => f());
      unReady.then((f) => f());
      unLoad.then((f) => f());
      unFailed.then((f) => f());
      unRemote.then((f) => f());
    };
  }, [retry]);

  const modelLabel = (config.model_name || '')
    .replace(/^ggml-/, '')
    .replace(/\.bin$/, '')
    .replace(/-/g, ' ');

  const toggleMode = config.trigger_mode === 'toggle';
  const hotkey = formatHotkey(toggleMode ? config.toggle_hotkey : config.hold_hotkey);
  const cloudConfigured = !!config.cloud_base_url.trim() && !!config.cloud_api_key.trim();
  const stateLabel = destination === 'receive' ? 'Receive mode · microphone off'
    : licenseError ? 'Membership status unavailable'
    : !license ? 'Checking membership…'
    : !allowsDictation(license) ? 'Membership required'
    : config.use_cloud ? (cloudConfigured ? 'Cloud speech configured · connection not tested' : 'Cloud speech setup incomplete')
    : engineError ? 'Model needs attention'
    : engineReady === null ? 'Checking model…'
    : engineReady ? 'Local speech model ready' : 'Speech model not ready';
  const ready = destination !== 'receive' && allowsDictation(license) && !config.use_cloud && engineReady && !engineError;
  const notice = membershipNotice(license);
  const empty = stats !== null && stats.dictations_total === 0;
  // A typed name is shown as typed; the OS full name is trimmed to its first
  // word so "Welcome back, João Francisco Vieira" does not wrap the title.
  const name = (config.display_name ?? '').trim() || osName.trim().split(/\s+/)[0] || '';

  return (
    <div className="home">
      <div className="home-top">
        {notice && (
          <div className="banner">
            <span>{notice}</span>
            <button className="btn btn-primary" type="button" onClick={() => onOpenTab('membership')}>
              {license?.kind === 'unlinked' ? 'Connect wallet' : 'Membership'}
            </button>
          </div>
        )}

        <h2 className="settings-panel-title home-greeting">
          {name ? `Welcome back, ${name}` : 'Welcome back'}
        </h2>
        <p className="settings-panel-sub">
          {destination === 'receive' ? 'Receive dictations from your paired computer.'
            : empty ? 'Set up your dictation preferences, then start talking.' : 'Here is your week so far.'}
        </p>

        <div className="home-stats">
          <div className="home-stat">
            <div className="home-stat-value">{stats?.words_7d ?? '—'}</div>
            <div className="home-stat-label">Words this week</div>
          </div>
          <div className="home-stat">
            <div className="home-stat-value">{stats?.dictations_7d ?? '—'}</div>
            <div className="home-stat-label">Dictations this week</div>
          </div>
          <div className="home-stat">
            <div className="home-stat-value">{stats?.streak_days ?? '—'}</div>
            <div className="home-stat-label">Day streak</div>
          </div>
        </div>

        <div className="home-status">
          {destination !== 'receive' && <>
            {config.use_cloud ? <span>{config.cloud_transcription_model || 'whisper-large-v3-turbo'}</span> : (
              <button className="home-link" type="button" onClick={() => onOpenTab('models')}>
                {modelLabel || 'Choose a model'}
              </button>
            )}
            <span aria-hidden="true">·</span>
            <span>{toggleMode ? 'Tap' : 'Hold'} {hotkey}</span>
            <span aria-hidden="true">·</span>
          </>}
          <span className={`home-state${ready ? ' ready' : ''}`} role="status">
            {stateLabel}
          </span>
        </div>
        <p className="home-destination">
          {destination === 'receive' ? 'Text arrives from your paired computer. Keep the destination field focused.'
            : destination === 'send' ? 'Destination: paired computer. Keep its destination field focused.'
            : destination === 'local' ? 'Destination: this computer.' : 'Destination status unavailable. Check Settings → General.'}
        </p>
        {destination !== 'receive' && <p className="field-hint">Speech recognition: {config.use_cloud ? 'cloud API' : 'local'}. Cleanup: {config.ai_cleanup_enabled ? config.ai_cleanup_engine === 'chatgpt' ? 'ChatGPT subscription' : config.ai_cleanup_engine === 'claude' ? 'Claude subscription' : config.ai_cleanup_engine === 'local' ? 'local' : 'cloud API' : 'off (app profiles may override)'}. Translation: {!config.output_language || config.output_language === 'same' ? 'off' : config.translation_engine === 'chatgpt' ? 'ChatGPT subscription' : config.translation_engine === 'claude' ? 'Claude subscription' : config.translation_engine === 'cloud' ? 'cloud API' : 'local'}. Text processing readiness is separate from speech readiness.</p>}
        {(statsError || licenseError || (!config.use_cloud && engineError)) && (
          <div className="inline-error" role="alert">
            <span>{statsError ? 'Your activity could not be loaded.' : 'A status check failed.'}</span>
            <button className="btn btn-secondary" type="button" onClick={() => setRetry((value) => value + 1)}>Check again</button>
          </div>
        )}

        {empty && destination !== 'receive' && (
          <p className="home-hint">
            {toggleMode ? 'Tap' : 'Hold'} <span className="kbd">{hotkey}</span> and talk.
            {toggleMode ? ' Tap again to finish.' : ' Release to finish.'} A copy of each dictation appears here.
          </p>
        )}
      </div>

      <HistoryList config={config} heading={false} onHistoryCleared={loadStats} />
    </div>
  );
}
