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
  const [engineReady, setEngineReady] = useState(true);
  const [license, setLicense] = useState<LicenseStatus | null>(null);
  const [osName, setOsName] = useState('');

  const loadStats = () => {
    // The streak's day boundaries are local midnight, which only the
    // webview knows; getTimezoneOffset() is minutes behind UTC.
    invoke<HistoryStats>('get_history_stats', { tzOffsetMin: new Date().getTimezoneOffset() })
      .then(setStats)
      .catch(() => {});
  };

  useEffect(() => {
    loadStats();
    invoke<boolean>('engine_ready')
      .then(setEngineReady)
      .catch(() => setEngineReady(true));
    invoke<LicenseStatus>('license_status').then(setLicense).catch(() => {});
    invoke<string>('os_display_name').then(setOsName).catch(() => {});
    const unLicense = listen<LicenseStatus>('license-changed', (e) => setLicense(e.payload));
    const unAdded = listen('history-added', () => loadStats());
    const unReady = listen('model-active', () => setEngineReady(true));
    const unLoad = listen('model-loading', () => setEngineReady(false));
    return () => {
      unLicense.then((f) => f());
      unAdded.then((f) => f());
      unReady.then((f) => f());
      unLoad.then((f) => f());
    };
  }, []);

  const modelLabel = (config.model_name || '')
    .replace(/^ggml-/, '')
    .replace(/\.bin$/, '')
    .replace(/-/g, ' ');

  const hotkey = formatHotkey(config.hold_hotkey);
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
          {empty ? 'Your first dictation is one keypress away.' : 'Here is your week so far.'}
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
          <button className="home-link" type="button" onClick={() => onOpenTab('models')}>
            {modelLabel || 'No model'}
          </button>
          <span aria-hidden="true">·</span>
          <span>{hotkey}</span>
          <span aria-hidden="true">·</span>
          <span className={`home-state${!allowsDictation(license) ? ' locked' : engineReady ? ' ready' : ''}`}>
            {!allowsDictation(license) ? 'Locked' : engineReady ? 'Ready' : 'Loading model…'}
          </span>
        </div>

        {empty && (
          <p className="home-hint">
            Hold <span className="kbd">{hotkey}</span> and talk. Your words land in whatever field is
            focused, and a copy shows up here so you can grab it again.
          </p>
        )}
      </div>

      <HistoryList config={config} heading={false} />
    </div>
  );
}
