import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Config } from './types';
import { formatHotkey } from './types';

interface HistoryEntry {
  text: string;
  timestamp: number;
  language: string | null;
}

interface HistoryStats {
  dictations_7d: number;
  words_7d: number;
  dictations_total: number;
  words_total: number;
}

function formatTime(ms: number): string {
  try {
    return new Date(ms).toLocaleString();
  } catch {
    return '';
  }
}

interface HomePanelProps {
  config: Config;
  onOpenTab: (tab: 'vocabulary' | 'models') => void;
}

export function HomePanel({ config, onOpenTab }: HomePanelProps) {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [stats, setStats] = useState<HistoryStats | null>(null);
  const [copied, setCopied] = useState<number | null>(null);
  const [filter, setFilter] = useState('');
  const [engineReady, setEngineReady] = useState(true);

  const load = () => {
    invoke<HistoryEntry[]>('get_history')
      .then((list) => setEntries([...list].reverse()))
      .catch(() => {});
    invoke<HistoryStats>('get_history_stats')
      .then(setStats)
      .catch(() => {});
  };

  useEffect(() => {
    load();
    invoke<boolean>('engine_ready')
      .then(setEngineReady)
      .catch(() => setEngineReady(true));
    const unAdded = listen<HistoryEntry>('history-added', () => load());
    const unReady = listen('model-active', () => setEngineReady(true));
    const unLoad = listen('model-loading', () => setEngineReady(false));
    return () => {
      unAdded.then((f) => f());
      unReady.then((f) => f());
      unLoad.then((f) => f());
    };
  }, []);

  const visible = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter((e) => e.text.toLowerCase().includes(q));
  }, [entries, filter]);

  const copy = (text: string, i: number) => {
    invoke('copy_to_clipboard', { text })
      .then(() => { setCopied(i); setTimeout(() => setCopied(null), 1200); })
      .catch(() => {});
  };

  const clearAll = () => {
    invoke('clear_history').then(() => {
      setEntries([]);
      setStats({ dictations_7d: 0, words_7d: 0, dictations_total: 0, words_total: 0 });
    }).catch(() => {});
  };

  const modelLabel = (config.model_name || '')
    .replace(/^ggml-/, '')
    .replace(/\.bin$/, '')
    .replace(/-/g, ' ');

  return (
    <div>
      <h2 className="settings-panel-title">Home</h2>
      <p className="settings-panel-sub">
        Last {Math.max(stats?.dictations_total ?? entries.length, 0)} dictations on this device.
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
          <div className="home-stat-value">{stats?.dictations_total ?? '—'}</div>
          <div className="home-stat-label">Stored</div>
        </div>
      </div>

      <div className="home-status">
        <span>{modelLabel || 'No model'}</span>
        <span>·</span>
        <span>{formatHotkey(config.hold_hotkey)}</span>
        <span>·</span>
        <span>{engineReady ? 'Ready' : 'Loading model…'}</span>
      </div>

      <div className="home-jumps">
        <button className="btn btn-secondary" type="button" onClick={() => onOpenTab('vocabulary')}>
          Vocabulary
        </button>
        <button className="btn btn-secondary" type="button" onClick={() => onOpenTab('models')}>
          Models
        </button>
      </div>

      {entries.length > 0 && (
        <div className="home-list-tools">
          <input
            className="input"
            value={filter}
            placeholder="Filter dictations…"
            onChange={(e) => setFilter(e.target.value)}
          />
          <button className="btn btn-ghost" type="button" onClick={clearAll}>
            Clear history
          </button>
        </div>
      )}

      {entries.length === 0 ? (
        <p className="history-empty">
          Hold {formatHotkey(config.hold_hotkey)} and talk — your words show up here.
        </p>
      ) : visible.length === 0 ? (
        <p className="history-empty">No dictations match that filter.</p>
      ) : (
        <div className="history-list">
          {visible.map((e, i) => (
            <div className="history-item" key={`${e.timestamp}-${i}`}>
              <div className="history-item-text">{e.text}</div>
              <div className="history-item-meta">
                <span>{formatTime(e.timestamp)}</span>
                {e.language && <span>· {e.language}</span>}
                <span className="history-item-actions">
                  <button className="history-copy" onClick={() => copy(e.text, i)}>
                    {copied === i ? 'Copied ✓' : 'Copy'}
                  </button>
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
