import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface UpdateInfo {
  version: string;
  notes: string | null;
}

/**
 * Thin banner pinned to the top of the Settings window when a new build has been downloaded in the background
 * and is waiting for a restart. The Rust side emits `update-downloaded` after a background check; failures
 * are log-only and never reach this banner.
 */
export function UpdateBanner() {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    const un = listen<UpdateInfo>('update-downloaded', (e) => setInfo(e.payload));
    return () => { un.then((f) => f()); };
  }, []);

  if (!info) return null;

  const restart = async () => {
    setInstalling(true);
    setError('');
    try {
      await invoke('restart_and_apply');
      // Process exits on success; this line only runs if install is async on this platform.
    } catch (e) {
      setInstalling(false);
      setError(String(e));
    }
  };

  return (
    <div className="update-banner">
      <span className="update-banner-text">
        Update to v{info.version} ready
      </span>
      <button className="btn btn-primary update-banner-btn" onClick={restart} disabled={installing}>
        {installing ? 'Restarting…' : 'Restart now'}
      </button>
      <button className="btn btn-secondary update-banner-btn" onClick={() => setInfo(null)} disabled={installing}>
        Later
      </button>
      {error && <span className="field-hint">{error}</span>}
    </div>
  );
}
