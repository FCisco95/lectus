import { useEffect, useState } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
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

  useEffect(() => {
    const unlisten = listen('pipeline-start', async () => {
      try {
        await invoke('run_pipeline');
      } catch (e) {
        console.error('pipeline error:', e);
      }
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  if (windowLabel === 'pill') return <Pill state={appState} />;
  return <Settings />;
}
