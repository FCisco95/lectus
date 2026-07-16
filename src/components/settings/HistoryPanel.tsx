import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

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

export function HistoryPanel() {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [copied, setCopied] = useState<number | null>(null);

  const load = () => {
    invoke<HistoryEntry[]>('get_history')
      .then((list) => setEntries([...list].reverse())) // newest first
      .catch(() => {});
  };

  useEffect(() => {
    load();
    const unlisten = listen<HistoryEntry>('history-added', () => load());
    return () => { unlisten.then((f) => f()); };
  }, []);

  const copy = (text: string, i: number) => {
    invoke('copy_to_clipboard', { text })
      .then(() => { setCopied(i); setTimeout(() => setCopied(null), 1200); })
      .catch(() => {});
  };

  const clearAll = () => {
    invoke('clear_history').then(() => setEntries([])).catch(() => {});
  };

  return (
    <div>
      <h2 className="settings-panel-title">History</h2>
      <p className="settings-panel-sub">Your last {Math.max(entries.length, 0)} dictations, kept on this device.</p>

      {entries.length > 0 && (
        <button className="btn btn-ghost" type="button" onClick={clearAll} style={{ marginBottom: 14 }}>
          Clear history
        </button>
      )}

      {entries.length === 0 ? (
        <p className="history-empty">No dictations yet. Start talking and they’ll show up here.</p>
      ) : (
        <div className="history-list">
          {entries.map((e, i) => (
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
