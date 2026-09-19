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

function formatTime(ms: number): string {
  try {
    return new Date(ms).toLocaleString();
  } catch {
    return '';
  }
}

interface HistoryListProps {
  config: Config;
  /** Page title + subtitle. Off when the list sits under Home's KPIs. */
  heading?: boolean;
}

/** Filter, copy, and clear for past dictations. Used on Home and the Dictations surface. */
export function HistoryList({ config, heading = true }: HistoryListProps) {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [copied, setCopied] = useState<number | null>(null);
  const [filter, setFilter] = useState('');

  const load = () => {
    invoke<HistoryEntry[]>('get_history')
      .then((list) => setEntries([...list].reverse()))
      .catch(() => {});
  };

  useEffect(() => {
    load();
    const unAdded = listen<HistoryEntry>('history-added', () => load());
    return () => { unAdded.then((f) => f()); };
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
    invoke('clear_history').then(() => setEntries([])).catch(() => {});
  };

  // Home already has an empty-state hint above the KPIs; don't repeat it.
  if (!heading && entries.length === 0) return null;

  return (
    <div className={`dictations${heading ? '' : ' dictations-embedded'}`}>
      {heading && (
        <div className="dictations-head">
          <h2 className="settings-panel-title">Dictations</h2>
          <p className="settings-panel-sub">
            {entries.length === 0
              ? 'Kept on this device only.'
              : `Your last ${entries.length} ${entries.length === 1 ? 'dictation' : 'dictations'}, kept on this device.`}
          </p>
        </div>
      )}

      {entries.length > 0 && (
        <div className="dictations-tools">
          <input
            className="input"
            value={filter}
            placeholder="Filter dictations…"
            aria-label="Filter dictations"
            onChange={(e) => setFilter(e.target.value)}
          />
          <button className="btn btn-ghost" type="button" onClick={clearAll}>
            Clear history
          </button>
        </div>
      )}

      <div className="dictations-scroll">
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
                    <button className="history-copy" type="button" onClick={() => copy(e.text, i)}>
                      {copied === i ? 'Copied ✓' : 'Copy'}
                    </button>
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
