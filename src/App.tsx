import { useEffect, useState } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Pill } from './components/Pill';
import { Settings } from './components/Settings';
import { Onboarding } from './components/Onboarding';
import { IS_MAC } from './components/settings/types';
import type { Config } from './components/settings/types';
import './styles/tokens.css';

type AppState = 'idle' | 'recording' | 'transcribing';

export default function App() {
  const [appState, setAppState] = useState<AppState>('idle');
  const [windowLabel] = useState(() => getCurrentWebviewWindow().label);
  const [onboardingDone, setOnboardingDone] = useState<boolean | null>(null);

  useEffect(() => {
    if (windowLabel === 'pill') return;
    const load = () =>
      invoke<Config>('get_config')
        .then((c) => setOnboardingDone(c.onboarding_completed))
        .catch(() => setOnboardingDone(true));
    load();
    // Settings stays mounted while hidden. Re-read on focus so a first
    // fetch that raced defaults cannot leave onboarding stuck on screen.
    const win = getCurrentWebviewWindow();
    const unFocus = win.listen('tauri://focus', () => { load(); });
    return () => { unFocus.then((f) => f()); };
  }, [windowLabel]);

  useEffect(() => {
    const unlisten = listen<string>('state-changed', (e) => {
      setAppState(e.payload as AppState);
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  // The pill window is transparent; tag <html> so pill.css can neutralise the
  // global opaque background for this window only. The platform class lets the
  // stylesheets pick native tokens (SF/mac radii vs Segoe/Fluent).
  useEffect(() => {
    if (windowLabel === 'pill') {
      document.documentElement.classList.add('pill-window');
    }
    document.documentElement.classList.add(IS_MAC ? 'platform-mac' : 'platform-win');
  }, [windowLabel]);

  // Theme: "system" resolves live against the OS media query; "light"/"dark"
  // pin the attribute regardless of OS. Settings persists `theme` in Config —
  // read it directly here (rather than threading through Settings' own
  // fetch) so the pill window also gets the right attribute.
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    let theme: Config['theme'] = 'system';

    const apply = () => {
      const resolved = theme === 'system' ? (media.matches ? 'dark' : 'light') : theme;
      document.documentElement.setAttribute('data-theme', resolved);
    };

    invoke<Config>('get_config')
      .then((c) => { theme = c.theme ?? 'system'; apply(); })
      .catch(() => apply());

    media.addEventListener('change', apply);
    const unlisten = listen<string>('theme-changed', (e) => {
      theme = (e.payload as Config['theme']) ?? 'system';
      apply();
    });
    return () => {
      media.removeEventListener('change', apply);
      unlisten.then((f) => f());
    };
  }, []);

  if (windowLabel === 'pill') return <Pill state={appState} />;
  if (onboardingDone === null) return null;
  if (!onboardingDone) return <Onboarding onComplete={() => setOnboardingDone(true)} />;
  return <Settings />;
}
