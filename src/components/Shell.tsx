import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getVersion } from '@tauri-apps/api/app';
import '../styles/settings.css';
import { useConfig } from './settings/useConfig';
import { Sidebar } from './Sidebar';
import type { SurfaceId } from './Sidebar';
import { SettingsModal } from './SettingsModal';
import type { SettingsTab } from './SettingsModal';
import { HomePanel } from './settings/HomePanel';
import { DictationsPanel } from './settings/DictationsPanel';
import { DictionaryPanel } from './settings/DictionaryPanel';
import { MembershipPanel } from './settings/MembershipPanel';
import { UpdateBanner } from './update-banner';
import { WhatsNewModal } from './WhatsNewModal';
import type { WhatsNew } from './WhatsNewModal';

/** The app shell: sidebar of surfaces on the window background, content in one
 *  card. Everything configurable opens in a modal on top (SettingsModal). */
export function Shell() {
  const { config, update, status, statusKind } = useConfig();
  const [surface, setSurface] = useState<SurfaceId>('home');
  const [settingsTab, setSettingsTab] = useState<SettingsTab | null>(null);
  const [version, setVersion] = useState('');
  const [whatsNew, setWhatsNew] = useState<WhatsNew | null>(null);
  const columnsRef = useRef<HTMLDivElement>(null);
  const closeSettings = useCallback(() => setSettingsTab(null), []);
  const closeWhatsNew = useCallback(() => setWhatsNew(null), []);
  const blocking = settingsTab !== null || whatsNew !== null;

  // Set inert as a DOM property when the overlay opens/closes — not as a
  // JSX boolean on every config keystroke. Re-applying inert mid-type moves
  // focus to the dialog and selects the whole General card.
  useEffect(() => {
    const el = columnsRef.current;
    if (el) el.inert = blocking;
  }, [blocking]);

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
    // Release notes parked by the previous build's "Restart now"; the
    // command deletes them, so this shows exactly once per update.
    invoke<WhatsNew | null>('take_whats_new_notes')
      .then((n) => { if (n) setWhatsNew(n); })
      .catch(() => {});
    const unHome = listen('open-home', () => setSurface('home'));
    // A refused dictation: go straight to the screen that can fix it, and get
    // the modal out of the way if it happened to be open.
    const unBlocked = listen('license-blocked', () => {
      setSettingsTab(null);
      setSurface('membership');
    });
    return () => {
      unHome.then((f) => f());
      unBlocked.then((f) => f());
    };
  }, []);

  if (!config) {
    return <div className="settings-app" style={{ padding: 24 }}>{status || 'Loading…'}</div>;
  }

  // Home's shortcut buttons: two are surfaces, one lives in the modal now.
  const jump = (target: 'vocabulary' | 'models' | 'membership') => {
    if (target === 'models') setSettingsTab('models');
    else setSurface(target);
  };

  return (
    <div className="settings-app">
      <div className="shell-columns" ref={columnsRef}>
        <Sidebar
          active={surface}
          onSelect={setSurface}
          onOpenSettings={() => setSettingsTab('general')}
          version={version}
        />

        <main className="surface">
          <UpdateBanner />
          <div className="surface-body">
            {surface === 'home' && <HomePanel config={config} onOpenTab={jump} />}
            {surface === 'dictations' && <DictationsPanel config={config} />}
            {surface === 'vocabulary' && <DictionaryPanel config={config} update={update} />}
            {surface === 'membership' && <MembershipPanel />}
          </div>
        </main>
      </div>

      {status && <div className={`settings-autosave-status ${statusKind}`}>{status}</div>}

      {whatsNew && !settingsTab && (
        <WhatsNewModal info={whatsNew} onClose={closeWhatsNew} />
      )}

      {settingsTab && (
        <SettingsModal
          config={config}
          update={update}
          initialTab={settingsTab}
          version={version}
          onClose={closeSettings}
        />
      )}
    </div>
  );
}
