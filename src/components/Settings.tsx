import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { getVersion } from '@tauri-apps/api/app';
import '../styles/settings.css';
import type { Config } from './settings/types';
import { Icon } from './Icon';
import type { IconName } from './Icon';
import { HomePanel } from './settings/HomePanel';
import { GeneralPanel } from './settings/GeneralPanel';
import { DictationPanel } from './settings/DictationPanel';
import { DictionaryPanel } from './settings/DictionaryPanel';
import { AIPanel } from './settings/AIPanel';
import { ModelsPanel } from './settings/ModelsPanel';
import { UpdateBanner } from './update-banner';

type TabId = 'home' | 'general' | 'dictation' | 'vocabulary' | 'ai' | 'models';

const TABS: Array<{ id: TabId; label: string; icon: IconName }> = [
  { id: 'home', label: 'Home', icon: 'home' },
  { id: 'general', label: 'General', icon: 'general' },
  { id: 'dictation', label: 'Dictation', icon: 'dictation' },
  { id: 'vocabulary', label: 'Vocabulary', icon: 'vocabulary' },
  { id: 'ai', label: 'AI', icon: 'ai' },
  { id: 'models', label: 'Models', icon: 'models' },
];

export function Settings() {
  const [config, setConfig] = useState<Config | null>(null);
  const [tab, setTab] = useState<TabId>('home');
  const [status, setStatus] = useState('');
  const [statusKind, setStatusKind] = useState<'' | 'ok' | 'err'>('');
  const [version, setVersion] = useState('');
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const statusTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const configRef = useRef<Config | null>(null);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  useEffect(() => {
    const persist = async (next: Config) => {
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
    };

    const flush = async () => {
      if (saveTimer.current) {
        clearTimeout(saveTimer.current);
        saveTimer.current = null;
        if (configRef.current) await persist(configRef.current);
      }
    };

    const load = () =>
      invoke<Config>('get_config')
        .then((c) => {
          configRef.current = c;
          setConfig(c);
        })
        .catch((e) => { setStatus(`Load error: ${e}`); setStatusKind('err'); });
    load();
    getVersion().then(setVersion).catch(() => {});

    // Settings is hidden, not unmounted. Flush a pending debounce, then
    // re-read so a model picked via select_model isn't overwritten by a
    // stale auto-save snapshot.
    const win = getCurrentWebviewWindow();
    const unFocus = win.listen('tauri://focus', () => {
      flush().then(load);
    });
    const unBlur = win.listen('tauri://blur', () => { flush(); });
    const unModel = listen<string>('model-active', (e) => {
      setConfig((c) => {
        if (!c) return c;
        const next = { ...c, model_name: e.payload };
        configRef.current = next;
        return next;
      });
    });
    const unHome = listen('open-home', () => setTab('home'));
    return () => {
      unFocus.then((f) => f());
      unBlur.then((f) => f());
      unModel.then((f) => f());
      unHome.then((f) => f());
    };
  }, []);

  if (!config) {
    return <div style={{ padding: 24, fontFamily: 'system-ui' }}>{status || 'Loading…'}</div>;
  }

  // Auto-save: every change persists after a short debounce (no Save button).
  const update = (patch: Partial<Config>) => {
    const next = { ...config, ...patch };
    configRef.current = next;
    setConfig(next);
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(async () => {
      saveTimer.current = null;
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
        {tab === 'home' && <HomePanel config={config} onOpenTab={setTab} />}
        {tab === 'general' && <GeneralPanel config={config} update={update} />}
        {tab === 'dictation' && <DictationPanel config={config} update={update} />}
        {tab === 'vocabulary' && <DictionaryPanel config={config} update={update} />}
        {tab === 'ai' && <AIPanel config={config} update={update} />}
        {tab === 'models' && <ModelsPanel config={config} update={update} />}

        {status && <div className={`settings-autosave-status ${statusKind}`}>{status}</div>}
      </main>
    </div>
  );
}
