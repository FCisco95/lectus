import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import '../styles/settings.css';
import type { Config } from './settings/types';
import { Icon } from './Icon';
import type { IconName } from './Icon';
import { GeneralPanel } from './settings/GeneralPanel';
import { DictationPanel } from './settings/DictationPanel';
import { DictionaryPanel } from './settings/DictionaryPanel';
import { HistoryPanel } from './settings/HistoryPanel';
import { AIPanel } from './settings/AIPanel';
import { ModelsPanel } from './settings/ModelsPanel';
import { UpdateBanner } from './update-banner';

type TabId = 'general' | 'dictation' | 'vocabulary' | 'ai' | 'history' | 'models';

const TABS: Array<{ id: TabId; label: string; icon: IconName }> = [
  { id: 'general', label: 'General', icon: 'general' },
  { id: 'dictation', label: 'Dictation', icon: 'dictation' },
  { id: 'vocabulary', label: 'Vocabulary', icon: 'vocabulary' },
  { id: 'ai', label: 'AI Cleanup', icon: 'ai' },
  { id: 'history', label: 'History', icon: 'history' },
  { id: 'models', label: 'Models & About', icon: 'models' },
];

export function Settings() {
  const [config, setConfig] = useState<Config | null>(null);
  const [tab, setTab] = useState<TabId>('general');
  const [status, setStatus] = useState('');
  const [statusKind, setStatusKind] = useState<'' | 'ok' | 'err'>('');
  const [version, setVersion] = useState('');
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const statusTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    invoke<Config>('get_config')
      .then(setConfig)
      .catch((e) => { setStatus(`Load error: ${e}`); setStatusKind('err'); });
    getVersion().then(setVersion).catch(() => {});
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
        setStatus('Saved');
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
      <div className="settings-topbar" data-tauri-drag-region>
        <div className="settings-brand">
          <div className="settings-brand-orb" />
          <span className="settings-brand-name">Lectus</span>
        </div>
        <nav className="settings-nav">
          {TABS.map((t) => (
            <button
              key={t.id}
              className={`settings-nav-item${tab === t.id ? ' active' : ''}`}
              onClick={() => setTab(t.id)}
            >
              <span className="settings-nav-icon"><Icon name={t.icon} /></span>
              {t.label}
            </button>
          ))}
        </nav>
        <div className="settings-sidebar-spacer" />
        <span className="settings-version">v{version}</span>
      </div>

      <UpdateBanner />
      <main className="settings-content">
        {tab === 'general' && <GeneralPanel config={config} update={update} />}
        {tab === 'dictation' && <DictationPanel config={config} update={update} />}
        {tab === 'vocabulary' && <DictionaryPanel config={config} update={update} />}
        {tab === 'ai' && <AIPanel config={config} update={update} />}
        {tab === 'history' && <HistoryPanel />}
        {tab === 'models' && <ModelsPanel config={config} update={update} />}

        {status && <div className={`settings-autosave-status ${statusKind}`}>{status}</div>}
      </main>
    </div>
  );
}
