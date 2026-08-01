import { useEffect, useState } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { listen } from '@tauri-apps/api/event';
import { Pill } from './components/Pill';
import { Settings } from './components/Settings';
import { IS_MAC } from './components/settings/types';

type AppState = 'idle' | 'recording' | 'transcribing';

export default function App() {
  const [appState, setAppState] = useState<AppState>('idle');
  const [windowLabel] = useState(() => getCurrentWebviewWindow().label);

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

  if (windowLabel === 'pill') return <Pill state={appState} />;
  return <Settings />;
}
