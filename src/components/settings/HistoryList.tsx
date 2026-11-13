import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Config } from './types';
import { formatHotkey } from './types';

interface HistoryEntry {
  text: string;
  timestamp: number;
  language: string | null;
  source_text?: string | null;
  source_language?: string | null;
  translation_error?: string | null;
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
  onHistoryCleared?: () => void;
}

/** Filter, copy, and clear for past dictations. Used on Home and the Dictations surface. */
export function HistoryList({ config, heading = true, onHistoryCleared }: HistoryListProps) {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [copied, setCopied] = useState<string | null>(null);
  const [filter, setFilter] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [confirmClear, setConfirmClear] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [cleared, setCleared] = useState(false);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const clearRef = useRef<HTMLButtonElement>(null);
  const emptyRef = useRef<HTMLParagraphElement>(null);

  const load = () => {
    invoke<HistoryEntry[]>('get_history')
      .then((list) => { setEntries([...list].reverse()); setError(''); })
      .catch(() => setError('Could not load your dictations. Try again.'))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    load();
    const unAdded = listen<HistoryEntry>('history-added', () => load());
    return () => { unAdded.then((f) => f()); };
  }, []);

  useEffect(() => {
    if (confirmClear) cancelRef.current?.focus();
  }, [confirmClear]);

  const visible = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter((e) => e.text.toLowerCase().includes(q) || e.source_text?.toLowerCase().includes(q));
  }, [entries, filter]);

  const copy = (entry: HistoryEntry) => {
    const { text } = entry;
    invoke('copy_to_clipboard', { text })
      .then(() => { setCopied(`${entry.timestamp}-${text}`); setError(''); })
      .catch(() => setError('Could not copy this dictation. Try Copy again.'));
  };

  const clearAll = async () => {
    setClearing(true);
    setError('');
    try {
      await invoke('clear_history');
      setEntries([]);
      setFilter('');
      setConfirmClear(false);
      setCopied(null);
      setCleared(true);
      onHistoryCleared?.();
      requestAnimationFrame(() => emptyRef.current?.focus());
    } catch {
      setError('Could not confirm that history was cleared. Reload history to check.');
    } finally { setClearing(false); }
  };

  // Home already has an empty-state hint above the KPIs; don't repeat it.
  if (!heading && entries.length === 0 && !loading && !error && !cleared) return null;

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

      {error && <div className="inline-error" role="alert">
        <span>{error}</span>
        <button className="btn btn-secondary" type="button" onClick={load}>Reload history</button>
      </div>}

      {entries.length > 0 && (
        <div className="dictations-tools">
          <input
            className="input"
            value={filter}
            placeholder="Filter dictations…"
            aria-label="Filter dictations"
            onChange={(e) => setFilter(e.target.value)}
          />
          <button className="btn btn-ghost" type="button" ref={clearRef} disabled={confirmClear}
            onClick={() => setConfirmClear(true)}>
            Clear history
          </button>
        </div>
      )}
      {confirmClear && (
        <div className="history-confirm" role="group" aria-label="Confirm history deletion">
          <p>Delete all {entries.length} {entries.length === 1 ? 'dictation' : 'dictations'}? This cannot be undone.</p>
          <div className="history-confirm-actions">
            <button className="btn btn-secondary" type="button" ref={cancelRef} disabled={clearing}
              onClick={() => { setConfirmClear(false); requestAnimationFrame(() => clearRef.current?.focus()); }}>Cancel</button>
            <button className="btn btn-ghost" type="button" disabled={clearing} onClick={clearAll}>
              {clearing ? 'Deleting…' : 'Delete all dictations'}
            </button>
          </div>
        </div>
      )}
      <span className="sr-only" role="status">{copied ? 'Dictation copied to clipboard.' : ''}</span>

      <div className="dictations-scroll">
        {loading ? <p className="history-empty" role="status">Loading dictations…</p> : entries.length === 0 ? (
          <p className="history-empty" ref={emptyRef} tabIndex={-1}>
            {error ? 'Your history is unavailable until it can be reloaded.' : cleared ? 'History cleared.' : (
              <>No dictations yet. {config.trigger_mode === 'toggle' ? 'Tap' : 'Hold'}{' '}
                {formatHotkey(config.trigger_mode === 'toggle' ? config.toggle_hotkey : config.hold_hotkey)} to start.</>
            )}
          </p>
        ) : visible.length === 0 ? (
          <p className="history-empty">No dictations match that filter.</p>
        ) : (
          <div className="history-list">
            {visible.map((e, i) => (
              <div className="history-item" key={`${e.timestamp}-${i}`}>
                <div className="history-item-text">{e.text}</div>
                {e.translation_error && <p className="inline-error">Translation did not finish. This is the original text; it was not inserted.</p>}
                {e.source_text && e.source_text !== e.text && <details>
                  <summary>Original{e.source_language ? ` · ${e.source_language}` : ''}</summary>
                  <p className="history-item-text">{e.source_text}</p>
                  <button className="btn" type="button" onClick={() => copy({ ...e, text: e.source_text! })}>Copy original</button>
                </details>}
                <div className="history-item-meta">
                  <span>{formatTime(e.timestamp)}</span>
                  {e.language && <span>· {e.language}</span>}
                  <span className="history-item-actions">
                    <button className="history-copy" type="button" onClick={() => copy(e)}>
                      {copied === `${e.timestamp}-${e.text}` ? 'Copied' : 'Copy'}
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
