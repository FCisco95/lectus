import { useEffect, useRef, useState, type InputHTMLAttributes } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { isEnabled as autostartEnabled, enable as autostartEnable, disable as autostartDisable } from '@tauri-apps/plugin-autostart';
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
  const [osName, setOsName] = useState('');

  useEffect(() => {
    invoke<string[]>('list_input_devices').then(setDevices).catch(() => setDevices([]));
    invoke<string>('os_display_name').then(setOsName).catch(() => {});
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
          <button className="btn btn-primary" onClick={() => invoke('relaunch_app')}>
            Relaunch
          </button>
        </div>
      )}

      <div className="card">
        <div className="row">
          <div className="row-label">
            <b>Your name</b>
            <span>Used in the Home greeting. Leave empty to use your account name.</span>
          </div>
          <DebouncedInput
            className="input"
            type="text"
            style={{ maxWidth: 220 }}
            placeholder={osName || 'Your name'}
            aria-label="Your name"
            value={config.display_name ?? ''}
            onCommit={(display_name) => update({ display_name })}
          />
        </div>
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
            <b>Mute playback while dictating</b>
            <span>Music, videos and games go quiet while you hold the key, then return to the previous volume.</span>
          </div>
          <div
            className={`toggle${config.mute_while_dictating !== false ? '' : ' off'}`}
            onClick={() => update({ mute_while_dictating: config.mute_while_dictating === false })}
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
              <DebouncedInput
                className="input"
                type="text"
                style={{ maxWidth: 260 }}
                value={config.cloud_base_url}
                onCommit={(cloud_base_url) => update({ cloud_base_url })}
              />
            </div>
            <div className="row">
              <div className="row-label"><b>Cloud API key</b></div>
              <DebouncedInput
                className="input"
                type="password"
                style={{ maxWidth: 260 }}
                placeholder="gsk_…"
                value={config.cloud_api_key}
                onCommit={(cloud_api_key) => update({ cloud_api_key })}
              />
            </div>
          </>
        )}
      </div>
    </div>
  );
}

/** Local draft so each keystroke does not re-render the shell (which was
 *  stealing focus back to the dialog). Commits on pause or blur. */
function DebouncedInput({
  value,
  onCommit,
  commitMs = 400,
  ...rest
}: {
  value: string;
  onCommit: (value: string) => void;
  commitMs?: number;
} & InputHTMLAttributes<HTMLInputElement>) {
  const [draft, setDraft] = useState(value);
  const lastPushed = useRef(value);

  useEffect(() => {
    if (value !== lastPushed.current) {
      setDraft(value);
      lastPushed.current = value;
    }
  }, [value]);

  useEffect(() => {
    if (draft === lastPushed.current) return;
    const t = window.setTimeout(() => {
      lastPushed.current = draft;
      onCommit(draft);
    }, commitMs);
    return () => window.clearTimeout(t);
  }, [draft, commitMs, onCommit]);

  return (
    <input
      {...rest}
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={() => {
        if (draft !== lastPushed.current) {
          lastPushed.current = draft;
          onCommit(draft);
        }
      }}
    />
  );
}
