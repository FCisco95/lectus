import { useEffect, useRef, useState, type InputHTMLAttributes } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { isEnabled as autostartEnabled, enable as autostartEnable, disable as autostartDisable } from '@tauri-apps/plugin-autostart';
import { IS_MAC } from './types';
import type { PanelProps } from './types';
import { RemoteDictationPanel } from './RemoteDictationPanel';
import { CloudSettings } from './CloudSettings';
import { HelpTip } from '../HelpTip';

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
  const [launchBusy, setLaunchBusy] = useState(false);
  const [launchError, setLaunchError] = useState('');

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
    setLaunchBusy(true);
    setLaunchError('');
    try {
      if (launchAtLogin) await autostartDisable();
      else await autostartEnable();
      setLaunchAtLogin(!launchAtLogin);
    } catch {
      setLaunchError('Could not change launch at login. Try again.');
    } finally { setLaunchBusy(false); }
  };

  return (
    <div>
      <h2 className="settings-panel-title">General</h2>
      <p className="settings-panel-sub">Appearance, microphone and how text is inserted.</p>
      <HelpTip title="microphone & text insertion"
        why="Choose the input device Lectus hears and how finished text reaches the focused field."
        setup="Select your microphone, leave insertion on Auto to start, then test a short sentence in the destination app."
        privacy="After setup, Lectus keeps up to half a second of microphone audio in local memory; Receive mode turns local recording off, and cloud speech is a separate choice." />

      <RemoteDictationPanel />

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
          <div className="seg" role="group" aria-label="Appearance">
            {THEMES.map((t) => (
              <button
                type="button"
                key={t.id}
                aria-pressed={config.theme === t.id}
                className={config.theme === t.id ? 'on' : ''}
                onClick={() => update({ theme: t.id })}
              >
                {t.label}
              </button>
            ))}
          </div>
        </div>
        <div className="row">
          <div className="row-label"><b>Microphone</b><span>Input device used for dictation.</span></div>
          <select
            className="select"
            aria-label="Microphone"
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
            aria-label="Insert text using"
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
          <button type="button" role="switch" aria-label="Voice activity detection" aria-checked={config.vad_enabled}
            className={`toggle${config.vad_enabled ? '' : ' off'}`}
            onClick={() => update({ vad_enabled: !config.vad_enabled })}
          />
        </div>
        <div className="row">
          <div className="row-label">
            <b>Mute playback while dictating</b>
            <span>Music, videos and games go quiet while you hold the key, then return to the previous volume.</span>
          </div>
          <button type="button" role="switch" aria-label="Mute playback while dictating" aria-checked={config.mute_while_dictating !== false}
            className={`toggle${config.mute_while_dictating !== false ? '' : ' off'}`}
            onClick={() => update({ mute_while_dictating: config.mute_while_dictating === false })}
          />
        </div>
        <div className="row">
          <div className="row-label">
            <b>Launch at login</b>
            <span>Start Lectus in the background at sign-in.</span>
          </div>
          <button type="button" role="switch" aria-label="Launch at login" aria-checked={launchAtLogin}
            className={`toggle${launchAtLogin ? '' : ' off'}`} disabled={launchBusy} onClick={toggleLaunchAtLogin} />
        </div>
        {launchError && <p className="inline-error" role="alert">{launchError}</p>}
      </div>

      <div className="card">
        <div className="row">
          <div className="row-label">
            <b>Use cloud transcription</b>
            <span>Send audio to your configured speech provider.</span>
          </div>
          <button type="button" role="switch" aria-label="Use cloud transcription" aria-checked={config.use_cloud}
            className={`toggle${config.use_cloud ? '' : ' off'}`}
            onClick={() => update({ use_cloud: !config.use_cloud })}
          />
        </div>
        <CloudSettings config={config} update={update} />
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
