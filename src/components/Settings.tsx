import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import '../styles/settings.css';
import type { Config } from './settings/types';
import { GeneralPanel } from './settings/GeneralPanel';
import { DictationPanel } from './settings/DictationPanel';
import { LanguagePanel } from './settings/LanguagePanel';
import { DictionaryPanel } from './settings/DictionaryPanel';
import { HistoryPanel } from './settings/HistoryPanel';
import { AIPanel } from './settings/AIPanel';
import { AppsPanel } from './settings/AppsPanel';
import { ModelsPanel } from './settings/ModelsPanel';
import { AboutPanel } from './settings/AboutPanel';

type TabId = 'general' | 'dictation' | 'language' | 'dictionary' | 'history' | 'ai' | 'apps' | 'models' | 'about';

const TABS: Array<{ id: TabId; label: string; icon: string }> = [
  { id: 'general', label: 'General', icon: '⚙️' },
  { id: 'dictation', label: 'Dictation', icon: '🎙️' },
  { id: 'language', label: 'Language', icon: '🌐' },
  { id: 'dictionary', label: 'Dictionary', icon: '📖' },
  { id: 'history', label: 'History', icon: '🕘' },
  { id: 'ai', label: 'AI cleanup', icon: '✨' },
  { id: 'apps', label: 'Apps', icon: '🪟' },
  { id: 'models', label: 'Models', icon: '🧠' },
  { id: 'about', label: 'About', icon: '🦜' },
];

export function Settings() {
  const [config, setConfig] = useState<Config | null>(null);
  const [tab, setTab] = useState<TabId>('general');
  const [status, setStatus] = useState('');
  const [statusKind, setStatusKind] = useState<'' | 'ok' | 'err'>('');
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const statusTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    invoke<Config>('get_config')
      .then(setConfig)
      .catch((e) => { setStatus(`Load error: ${e}`); setStatusKind('err'); });
  }, []);

  if (!config) {
    return <div style={{ padding: 24, fontFamily: 'system-ui' }}>{status || 'Loading…'}</div>;
  }

  // Auto-save: every change persists after a short debounce (no Save button).
  const update = (patch: Partial<Config>) => {
    const next = { ...config, ...patch };
    setConfig(next);
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(async () => {
      try {
        await invoke('save_config', { newConfig: next });
        setStatus('Saved ✓');
        setStatusKind('ok');
        if (statusTimer.current) clearTimeout(statusTimer.current);
        statusTimer.current = setTimeout(() => setStatus(''), 1500);
      } catch (e) {
        setStatus(`${e}`);
        setStatusKind('err');
      }
    }, 500);
  };

  return (
    <div className="settings-app">
      <nav className="settings-sidebar">
        <div className="settings-brand">
          <div className="settings-brand-orb" />
          <span className="settings-brand-name">Lectus</span>
        </div>
        {TABS.map((t) => (
          <button
            key={t.id}
            className={`settings-nav-item${tab === t.id ? ' active' : ''}`}
            onClick={() => setTab(t.id)}
          >
            <span className="settings-nav-icon">{t.icon}</span>
            {t.label}
          </button>
        ))}
      </nav>

      <main className="settings-content">
        {tab === 'general' && <GeneralPanel config={config} update={update} />}
        {tab === 'dictation' && <DictationPanel config={config} update={update} />}
        {tab === 'language' && <LanguagePanel config={config} update={update} />}
        {tab === 'dictionary' && <DictionaryPanel config={config} update={update} />}
        {tab === 'history' && <HistoryPanel />}
        {tab === 'ai' && <AIPanel config={config} update={update} />}
        {tab === 'apps' && <AppsPanel config={config} update={update} />}
        {tab === 'models' && <ModelsPanel config={config} update={update} />}
        {tab === 'about' && <AboutPanel />}

        {status && <div className={`settings-autosave-status ${statusKind}`}>{status}</div>}
      </main>
    </div>
  );
}
