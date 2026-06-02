import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

// KeyboardEvent.code → Lectus config key string.
// MUST stay in sync with config_key_to_vk in src-tauri/src/hook/mod.rs.
const CODE_TO_KEY: Record<string, string> = {
  ControlRight: 'RControl',
  ControlLeft: 'LControl',
  ShiftRight: 'RShift',
  ShiftLeft: 'LShift',
  AltRight: 'RAlt',
  AltLeft: 'LAlt',
  F13: 'F13',
  F14: 'F14',
  F15: 'F15',
};

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
  const [capturing, setCapturing] = useState(false);

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
        <button
          type="button"
          onClick={() => { setCapturing(true); setStatus('Press a key…'); }}
          onKeyDown={(e) => {
            if (!capturing) return;
            e.preventDefault();
            const mapped = CODE_TO_KEY[e.code];
            setCapturing(false);
            if (!mapped) {
              setStatus(`Unsupported key (${e.code}). Try Right Ctrl, Shift, Alt, or F13-F15.`);
              return;
            }
            update({ hold_hotkey: mapped });
            setStatus(`Captured: ${mapped} — click Save`);
          }}
          style={{
            width: '100%',
            padding: 6,
            textAlign: 'left',
            background: capturing ? '#fffbe6' : '#fff',
            border: '1px solid #ccc',
            cursor: 'pointer',
          }}
        >
          {capturing ? 'Press a key…' : config.hold_hotkey}
        </button>
        <small style={{ color: '#888' }}>
          Click, then press your hold-to-talk key (hold to talk, release to stop).
        </small>
      </div>

      <button onClick={save} style={{ padding: '8px 16px' }}>
        Save
      </button>
      <span style={{ marginLeft: 12, fontSize: 13 }}>{status}</span>
    </div>
  );
}
