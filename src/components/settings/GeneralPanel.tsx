import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { isEnabled as autostartEnabled, enable as autostartEnable, disable as autostartDisable } from '@tauri-apps/plugin-autostart';
import { relaunch } from '@tauri-apps/plugin-process';
import { IS_MAC } from './types';
import type { PanelProps } from './types';

const THEMES: Array<{ id: 'system' | 'light' | 'dark'; label: string }> = [
  { id: 'system', label: 'System' },
  { id: 'light', label: 'Light' },
  { id: 'dark', label: 'Dark' },
];

export function GeneralPanel({ config, update }: PanelProps) {
  const [devices, setDevices] = useState<string[]>([]);
  const [launchAtLogin, setLaunchAtLogin] = useState(false);
  const [accessible, setAccessible] = useState(true);

  useEffect(() => {
    invoke<string[]>('list_input_devices').then(setDevices).catch(() => setDevices([]));
    autostartEnabled().then(setLaunchAtLogin).catch(() => {});
    if (IS_MAC) {
      const poll = () => invoke<boolean>('accessibility_status').then(setAccessible).catch(() => {});
      poll();
      const id = setInterval(poll, 2000);
      return () => clearInterval(id);
    }
  }, []);

  const toggleLaunchAtLogin = async () => {
    try {
      if (launchAtLogin) await autostartDisable();
      else await autostartEnable();
      setLaunchAtLogin(!launchAtLogin);
    } catch { /* leave the toggle as-is on failure */ }
  };

  return (
    <div>
      <h2 className="settings-panel-title">General</h2>
      <p className="settings-panel-sub">Appearance, microphone and how text is inserted.</p>

      {IS_MAC && !accessible && (
        <div className="banner">
          <span>⚠️</span>
          <span>
            <b>Accessibility permission is off.</b> The dictation hotkey won't work until it's
            granted and Lectus is relaunched.
          </span>
          <button className="btn" onClick={() => invoke('open_accessibility_settings')}>
            Open System Settings
          </button>
          <button className="btn btn-primary" onClick={() => relaunch()}>
            Relaunch
          </button>
        </div>
      )}

      <div className="card">
        <div className="row">
          <div className="row-label"><b>Appearance</b><span>Follow the system, or force light / dark.</span></div>
          <div className="seg">
            {THEMES.map((t) => (
              <span
                key={t.id}
                className={config.theme === t.id ? 'on' : ''}
                onClick={() => update({ theme: t.id })}
              >
                {t.label}
              </span>
            ))}
          </div>
        </div>
        <div className="row">
          <div className="row-label"><b>Microphone</b><span>Input device used for dictation.</span></div>
          <select
            className="select"
            value={config.input_device}
            onChange={(e) => update({ input_device: e.target.value })}
          >
            <option value="">System default</option>
            {devices.map((d) => (
              <option key={d} value={d}>{d}</option>
            ))}
          </select>
        </div>
        <div className="row">
          <div className="row-label">
            <b>Insert text using</b>
            <span>
              {IS_MAC
                ? 'Clipboard paste — your previous clipboard is restored afterwards.'
                : 'How transcribed text lands in the focused app.'}
            </span>
          </div>
          <select
            className="select"
            value={config.injection_mode}
            onChange={(e) => update({ injection_mode: e.target.value })}
          >
            <option value="auto">Auto (recommended)</option>
            {!IS_MAC && <option value="sendinput">Typed keystrokes</option>}
            <option value="clipboard">Clipboard paste</option>
          </select>
        </div>
      </div>

      <div className="card">
        <div className="row">
          <div className="row-label">
            <b>Voice activity detection</b>
            <span>Neural VAD trims silence before recognition — fewer "ghost words". Recommended on.</span>
          </div>
          <div
            className={`toggle${config.vad_enabled ? '' : ' off'}`}
            onClick={() => update({ vad_enabled: !config.vad_enabled })}
          />
        </div>
        <div className="row">
          <div className="row-label">
            <b>Launch at login</b>
            <span>Start Lectus in the background at sign-in.</span>
          </div>
          <div className={`toggle${launchAtLogin ? '' : ' off'}`} onClick={toggleLaunchAtLogin} />
        </div>
      </div>

      <div className="card">
        <div className="row">
          <div className="row-label">
            <b>Use cloud transcription</b>
            <span>Groq Whisper-large-v3-turbo. Faster and more accurate, needs internet + an API key.</span>
          </div>
          <div
            className={`toggle${config.use_cloud ? '' : ' off'}`}
            onClick={() => update({ use_cloud: !config.use_cloud })}
          />
        </div>
        {config.use_cloud && (
          <>
            <div className="row">
              <div className="row-label"><b>Cloud base URL</b><span>Any OpenAI-compatible endpoint.</span></div>
              <input
                className="input"
                type="text"
                style={{ maxWidth: 260 }}
                value={config.cloud_base_url}
                onChange={(e) => update({ cloud_base_url: e.target.value })}
              />
            </div>
            <div className="row">
              <div className="row-label"><b>Cloud API key</b></div>
              <input
                className="input"
                type="password"
                style={{ maxWidth: 260 }}
                placeholder="gsk_…"
                value={config.cloud_api_key}
                onChange={(e) => update({ cloud_api_key: e.target.value })}
              />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
