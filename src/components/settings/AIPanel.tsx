import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AppProfile, PanelProps } from './types';
import { IS_MAC, LANGUAGES } from './types';

const TONES = ['neutral', 'formal', 'casual', 'concise'];

interface CleanupModelStatus {
  downloaded: boolean;
  size_mb: number;
}

const emptyProfile = (): AppProfile => ({
  app_match: '',
  ai_cleanup_enabled: null,
  ai_cleanup_tone: null,
  language: null,
  injection_mode: null,
});

export function AIPanel({ config, update }: PanelProps) {
  const [claudeAvailable, setClaudeAvailable] = useState<boolean | null>(null);
  const [localModel, setLocalModel] = useState<CleanupModelStatus | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [captureHint, setCaptureHint] = useState('');
  const engine = config.ai_cleanup_engine || 'claude';
  const profiles = config.app_profiles ?? [];

  const setProfiles = (next: AppProfile[]) => update({ app_profiles: next });
  const patchProfile = (i: number, patch: Partial<AppProfile>) => {
    setProfiles(profiles.map((p, idx) => (idx === i ? { ...p, ...patch } : p)));
  };
  const captureApp = (i: number) => {
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

  useEffect(() => {
    invoke<boolean>('check_claude_cli').then(setClaudeAvailable).catch(() => setClaudeAvailable(false));
    invoke<CleanupModelStatus>('get_cleanup_model_status').then(setLocalModel).catch(() => setLocalModel(null));
  }, []);

  const downloadLocalModel = async () => {
    setDownloading(true);
    setDownloadError(null);
    try {
      await invoke('download_cleanup_model');
      const status = await invoke<CleanupModelStatus>('get_cleanup_model_status');
      setLocalModel(status);
    } catch (e) {
      setDownloadError(String(e));
    } finally {
      setDownloading(false);
    }
  };

  return (
    <div>
      <h2 className="settings-panel-title">AI cleanup</h2>
      <p className="settings-panel-sub">Optionally polish each transcript — punctuation, capitalization, fewer “um”s.</p>

      <label className="checkbox-row">
        <input
          type="checkbox"
          checked={config.ai_cleanup_enabled}
          onChange={(e) => update({ ai_cleanup_enabled: e.target.checked })}
        />
        <span className="checkbox-text">
          Enable AI cleanup
          <small>Runs after transcription. Adds a little latency.</small>
        </span>
      </label>

      {config.ai_cleanup_enabled && (
        <>
          <div className="field">
            <label className="field-label">Engine</label>
            <div className="radio-group">
              <label className={`radio-card${engine === 'local' ? ' selected' : ''}`}>
                <input
                  type="radio"
                  name="ai_engine"
                  checked={engine === 'local'}
                  onChange={() => update({ ai_cleanup_engine: 'local' })}
                />
                <span>
                  <div className="radio-title">
                    Local (offline){' '}
                    {localModel?.downloaded && <span className="badge ok">ready</span>}
                  </div>
                  <div className="radio-desc">
                    Gemma 3 on your GPU. Free, private, no internet — the fully local pipeline.
                  </div>
                </span>
              </label>
              <label className={`radio-card${engine === 'claude' ? ' selected' : ''}`}>
                <input
                  type="radio"
                  name="ai_engine"
                  checked={engine === 'claude'}
                  onChange={() => update({ ai_cleanup_engine: 'claude' })}
                />
                <span>
                  <div className="radio-title">
                    Claude Code CLI{' '}
                    {claudeAvailable === true && <span className="badge ok">detected</span>}
                    {claudeAvailable === false && <span className="badge warn">not found</span>}
                  </div>
                  <div className="radio-desc">Uses your Claude subscription — no API tokens. Best quality, ~1–3s.</div>
                </span>
              </label>
              <label className={`radio-card${engine === 'groq' ? ' selected' : ''}`}>
                <input
                  type="radio"
                  name="ai_engine"
                  checked={engine === 'groq'}
                  onChange={() => update({ ai_cleanup_engine: 'groq' })}
                />
                <span>
                  <div className="radio-title">Groq</div>
                  <div className="radio-desc">Sub-second. Uses your cloud API key (General tab).</div>
                </span>
              </label>
            </div>
            {engine === 'local' && localModel && !localModel.downloaded && (
              <div className="field-hint">
                Needs a one-time model download ({localModel.size_mb} MB).{' '}
                <button className="btn" onClick={downloadLocalModel} disabled={downloading}>
                  {downloading ? 'Downloading…' : 'Download model'}
                </button>
                {downloadError && <span className="badge warn">{downloadError}</span>}
              </div>
            )}
            {engine === 'claude' && claudeAvailable === false && (
              <div className="field-hint">
                Claude CLI not found on PATH. Install Claude Code, or switch to Groq. If unavailable,
                cleanup is skipped and the raw transcript is used.
              </div>
            )}
          </div>

          <div className="field">
            <label className="field-label">Tone</label>
            <select
              className="select"
              value={config.ai_cleanup_tone || 'neutral'}
              onChange={(e) => update({ ai_cleanup_tone: e.target.value })}
            >
              {TONES.map((t) => (
                <option key={t} value={t}>{t[0].toUpperCase() + t.slice(1)}</option>
              ))}
            </select>
          </div>
        </>
      )}

      <h2 className="settings-panel-title" style={{ marginTop: 32 }}>App profiles</h2>
      <p className="settings-panel-sub">
        Per-app overrides. When you dictate into a matching app, these replace the settings above.
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
              {!IS_MAC && <option value="sendinput">Typed keystrokes</option>}
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
