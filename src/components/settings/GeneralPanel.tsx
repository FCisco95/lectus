import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { PanelProps } from './types';

export function GeneralPanel({ config, update }: PanelProps) {
  const [devices, setDevices] = useState<string[]>([]);

  useEffect(() => {
    invoke<string[]>('list_input_devices').then(setDevices).catch(() => setDevices([]));
  }, []);

  return (
    <div>
      <h2 className="settings-panel-title">General</h2>
      <p className="settings-panel-sub">Microphone and where transcription runs.</p>

      <div className="field">
        <label className="field-label">Microphone</label>
        <select
          className="input"
          value={config.input_device}
          onChange={(e) => update({ input_device: e.target.value })}
        >
          <option value="">System default</option>
          {devices.map((d) => (
            <option key={d} value={d}>{d}</option>
          ))}
        </select>
        <div className="field-hint">Which input device Lectus records from.</div>
      </div>

      <label className="checkbox-row">
        <input
          type="checkbox"
          checked={config.use_cloud}
          onChange={(e) => update({ use_cloud: e.target.checked })}
        />
        <span className="checkbox-text">
          Use cloud transcription
          <small>Groq Whisper-large-v3-turbo. Faster and more accurate, needs internet + an API key. Off = fully local & private.</small>
        </span>
      </label>

      {config.use_cloud && (
        <>
          <div className="field">
            <label className="field-label">Cloud base URL</label>
            <input
              className="input"
              type="text"
              value={config.cloud_base_url}
              onChange={(e) => update({ cloud_base_url: e.target.value })}
            />
            <div className="field-hint">Any OpenAI-compatible endpoint.</div>
          </div>

          <div className="field">
            <label className="field-label">Cloud API key</label>
            <input
              className="input"
              type="password"
              placeholder="gsk_…"
              value={config.cloud_api_key}
              onChange={(e) => update({ cloud_api_key: e.target.value })}
            />
          </div>
        </>
      )}
    </div>
  );
}
