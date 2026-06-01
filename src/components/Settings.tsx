import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Config {
  model_path: string;
  use_cloud: boolean;
  cloud_base_url: string;
  cloud_api_key: string;
  hold_hotkey: string;
  toggle_hotkey: string;
}

export function Settings() {
  const [config, setConfig] = useState<Config | null>(null);
  const [status, setStatus] = useState('');

  useEffect(() => {
    invoke<Config>('get_config')
      .then(setConfig)
      .catch((e) => setStatus(`Load error: ${e}`));
  }, []);

  if (!config) {
    return <div style={{ padding: 24, fontFamily: 'system-ui' }}>{status || 'Loading…'}</div>;
  }

  const update = (patch: Partial<Config>) => setConfig({ ...config, ...patch });

  const save = async () => {
    setStatus('Saving…');
    try {
      await invoke('save_config', { newConfig: config });
      setStatus('Saved ✓');
    } catch (e) {
      setStatus(`Save error: ${e}`);
    }
  };

  return (
    <div style={{ padding: 24, fontFamily: 'system-ui', maxWidth: 432 }}>
      <h2 style={{ marginTop: 0 }}>Lectus Settings</h2>

      <label style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <input
          type="checkbox"
          checked={config.use_cloud}
          onChange={(e) => update({ use_cloud: e.target.checked })}
        />
        Use cloud transcription (Groq Whisper-large-v3-turbo)
      </label>

      <div style={{ marginBottom: 12 }}>
        <label style={{ display: 'block', fontSize: 13, marginBottom: 4 }}>Cloud base URL</label>
        <input
          type="text"
          value={config.cloud_base_url}
          onChange={(e) => update({ cloud_base_url: e.target.value })}
          style={{ width: '100%', padding: 6 }}
        />
      </div>

      <div style={{ marginBottom: 12 }}>
        <label style={{ display: 'block', fontSize: 13, marginBottom: 4 }}>Cloud API key</label>
        <input
          type="password"
          value={config.cloud_api_key}
          placeholder="gsk_…"
          onChange={(e) => update({ cloud_api_key: e.target.value })}
          style={{ width: '100%', padding: 6 }}
        />
      </div>

      <div style={{ marginBottom: 16 }}>
        <label style={{ display: 'block', fontSize: 13, marginBottom: 4 }}>Dictation hotkey</label>
        <input
          type="text"
          value={config.hold_hotkey}
          readOnly
          style={{ width: '100%', padding: 6, background: '#f0f0f0' }}
        />
        <small style={{ color: '#888' }}>Editable hotkey capture lands in a later phase.</small>
      </div>

      <button onClick={save} style={{ padding: '8px 16px' }}>
        Save
      </button>
      <span style={{ marginLeft: 12, fontSize: 13 }}>{status}</span>
    </div>
  );
}
