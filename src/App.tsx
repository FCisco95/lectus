import { useEffect, useState } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { listen } from '@tauri-apps/api/event';
import { Pill } from './components/Pill';
import { Settings } from './components/Settings';

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
  // global opaque background for this window only.
  useEffect(() => {
    if (windowLabel === 'pill') {
      document.documentElement.classList.add('pill-window');
    }
  }, [windowLabel]);

  if (windowLabel === 'pill') return <Pill state={appState} />;
  return <Settings />;
}
