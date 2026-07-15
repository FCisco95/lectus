import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AppProfile, PanelProps } from './types';
import { LANGUAGES } from './types';

const TONES = ['neutral', 'formal', 'casual', 'concise'];

const emptyProfile = (): AppProfile => ({
  app_match: '',
  ai_cleanup_enabled: null,
  ai_cleanup_tone: null,
  language: null,
  injection_mode: null,
});

export function AppsPanel({ config, update }: PanelProps) {
  const [captureHint, setCaptureHint] = useState('');
  const profiles = config.app_profiles ?? [];

  const setProfiles = (next: AppProfile[]) => update({ app_profiles: next });

  const patchProfile = (i: number, patch: Partial<AppProfile>) => {
    const next = profiles.map((p, idx) => (idx === i ? { ...p, ...patch } : p));
    setProfiles(next);
  };

  const captureApp = async (i: number) => {
    setCaptureHint('Focus the target app within 3 seconds…');
    setTimeout(async () => {
      try {
        const app = await invoke<string | null>('get_foreground_app');
        if (app) {
          patchProfile(i, { app_match: app.replace(/\.exe$/i, '') });
          setCaptureHint('');
        } else {
          setCaptureHint('Could not detect the focused app.');
        }
      } catch {
        setCaptureHint('Could not detect the focused app.');
      }
    }, 3000);
  };

  return (
    <div>
      <h2 className="settings-panel-title">Apps</h2>
      <p className="settings-panel-sub">
        Per-app behavior. When you dictate into a matching app, these overrides replace your global settings.
      </p>

      {profiles.map((p, i) => (
        <div className="model-card" key={i}>
          <div className="field">
            <label className="field-label">App name contains</label>
            <div style={{ display: 'flex', gap: 8 }}>
              <input
                className="input"
                type="text"
                placeholder="e.g. code, slack, chrome"
                value={p.app_match}
                onChange={(e) => patchProfile(i, { app_match: e.target.value })}
              />
              <button className="btn" onClick={() => captureApp(i)}>Detect</button>
              <button className="btn" onClick={() => setProfiles(profiles.filter((_, idx) => idx !== i))}>
                Remove
              </button>
            </div>
          </div>

          <div className="field">
            <label className="field-label">AI cleanup</label>
            <select
              className="input"
              value={p.ai_cleanup_enabled === null ? '' : String(p.ai_cleanup_enabled)}
              onChange={(e) =>
                patchProfile(i, {
                  ai_cleanup_enabled: e.target.value === '' ? null : e.target.value === 'true',
                })
              }
            >
              <option value="">Use global setting</option>
              <option value="true">On</option>
              <option value="false">Off</option>
            </select>
          </div>

          <div className="field">
            <label className="field-label">Tone</label>
            <select
              className="input"
              value={p.ai_cleanup_tone ?? ''}
              onChange={(e) => patchProfile(i, { ai_cleanup_tone: e.target.value || null })}
            >
              <option value="">Use global setting</option>
              {TONES.map((t) => (
                <option key={t} value={t}>{t[0].toUpperCase() + t.slice(1)}</option>
              ))}
            </select>
          </div>

          <div className="field">
            <label className="field-label">Language</label>
            <select
              className="input"
              value={p.language ?? ''}
              onChange={(e) => patchProfile(i, { language: e.target.value || null })}
            >
              <option value="">Use global setting</option>
              {LANGUAGES.map((l) => (
                <option key={l.code} value={l.code}>{l.label}</option>
              ))}
            </select>
          </div>

          <div className="field">
            <label className="field-label">Text insertion</label>
            <select
              className="input"
              value={p.injection_mode ?? ''}
              onChange={(e) => patchProfile(i, { injection_mode: e.target.value || null })}
            >
              <option value="">Use global setting</option>
              <option value="auto">Auto</option>
              <option value="sendinput">Typed keystrokes</option>
              <option value="clipboard">Clipboard paste</option>
            </select>
          </div>
        </div>
      ))}

      <button className="btn" onClick={() => setProfiles([...profiles, emptyProfile()])}>
        + Add app profile
      </button>
      {captureHint && <div className="field-hint">{captureHint}</div>}
    </div>
  );
}
