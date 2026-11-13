import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AppProfile, PanelProps } from './types';
import { IS_MAC, LANGUAGES } from './types';
import { HelpTip } from '../HelpTip';
import { AIConnections } from './AIConnections';
import { ModelLicensePanel } from './ModelLicensePanel';
import { ModelDownloadProgress, type DownloadState } from './ModelDownloadProgress';

const TONES = ['neutral', 'formal', 'casual', 'concise'];

interface CleanupModelStatus {
  name?: string;
  downloaded: boolean;
  size_mb: number;
  download?: DownloadState;
}

const emptyProfile = (): AppProfile => ({
  app_match: '',
  ai_cleanup_enabled: null,
  ai_cleanup_tone: null,
  language: null,
  injection_mode: null,
});

export function AIPanel({ config, update }: PanelProps) {
  const [localModel, setLocalModel] = useState<CleanupModelStatus | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const mounted = useRef(false);
  const [captureHint, setCaptureHint] = useState('');
  const engine = config.ai_cleanup_engine || 'claude';
  const profiles = config.app_profiles ?? [];
  const profilesRef = useRef(profiles);
  const updateRef = useRef(update);
  const captureTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const captureRevision = useRef(0);
  profilesRef.current = profiles;
  updateRef.current = update;

  const cancelCapture = () => {
    captureRevision.current += 1;
    if (captureTimer.current) clearTimeout(captureTimer.current);
    captureTimer.current = null;
  };
  const setProfiles = (next: AppProfile[]) => {
    // Editing or removing a profile cancels a pending detection, so its late
    // result cannot overwrite those edits or target a shifted array index.
    cancelCapture();
    setCaptureHint('');
    profilesRef.current = next;
    updateRef.current({ app_profiles: next });
  };
  const patchProfile = (i: number, patch: Partial<AppProfile>) => {
    setProfiles(profilesRef.current.map((p, idx) => (idx === i ? { ...p, ...patch } : p)));
  };
  const captureApp = (i: number) => {
    cancelCapture();
    const revision = captureRevision.current;
    const targetName = profilesRef.current[i]?.app_match;
    setCaptureHint('Focus the target app within 3 seconds…');
    captureTimer.current = setTimeout(async () => {
      captureTimer.current = null;
      try {
        const app = await invoke<string | null>('get_foreground_app');
        if (revision !== captureRevision.current) return;
        if (!profilesRef.current[i] || profilesRef.current[i].app_match !== targetName) {
          setCaptureHint('The profile changed. Select Detect to try again.');
          return;
        }
        if (app) {
          patchProfile(i, { app_match: app.replace(/\.exe$/i, '') });
          setCaptureHint('');
        } else {
          setCaptureHint('Could not detect the focused app.');
        }
      } catch {
        if (revision === captureRevision.current) setCaptureHint('Could not detect the focused app.');
      }
    }, 3000);
  };

  useEffect(() => {
    mounted.current = true;
    let active = true;
    const refresh = () => invoke<CleanupModelStatus>('get_cleanup_model_status')
      .then(status => { if (active) setLocalModel(status); })
      .catch(() => { if (active) setDownloadError('Could not check the cleanup model. Reopen this panel to retry.'); });
    refresh();
    const timer = window.setInterval(refresh, 1500);
    const unlisten = listen('model-download-state', refresh);
    return () => { mounted.current = false; active = false; window.clearInterval(timer); unlisten.then(stop => stop()); cancelCapture(); };
  }, []);

  const downloadLocalModel = async () => {
    setDownloading(true);
    setDownloadError(null);
    try {
      await invoke('download_cleanup_model');
      const status = await invoke<CleanupModelStatus>('get_cleanup_model_status');
      if (mounted.current) setLocalModel(status);
    } catch (e) {
      if (mounted.current) {
        try {
          const status = await invoke<CleanupModelStatus>('get_cleanup_model_status');
          if (mounted.current) {
            setLocalModel(status);
            if (!status.download?.paused && !status.download?.error) setDownloadError(String(e));
          }
        } catch { if (mounted.current) setDownloadError(String(e)); }
      }
    } finally {
      if (mounted.current) setDownloading(false);
    }
  };

  return (
    <div>
      <h2 className="settings-panel-title">AI cleanup</h2>
      <p className="settings-panel-sub">Optionally polish each transcript — punctuation, capitalization, fewer “um”s.</p>
      <HelpTip title="AI cleanup"
        why="Polish recognised words without changing the output language; translation is configured separately in Dictation."
        setup="Enable cleanup, select an engine and tone, then test the result; local models need a download and their first use can take longer."
        privacy="Local cleanup stays on-device; API and subscription routes send text outside Lectus and require your separate permission even with local speech recognition." />

      <label className="checkbox-row">
        <input
          type="checkbox"
          checked={config.ai_cleanup_enabled}
          onChange={(e) => update({ ai_cleanup_enabled: e.target.checked })}
        />
        <span className="checkbox-text">
          Enable AI cleanup
          <small>Runs after transcription. Adds processing time.</small>
        </span>
      </label>

      <AIConnections config={config} update={update} />

      <>
          <div className="field">
            <div className="field-label" id="cleanup-engine-label">Engine</div>
            <div className="radio-group" role="radiogroup" aria-labelledby="cleanup-engine-label">
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
                    {localModel?.downloaded && <span className="badge ok">downloaded</span>}
                  </div>
                  <div className="radio-desc">
                    Gemma 3 on your GPU after a one-time download. Cleanup runs on this device.
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
                    Claude subscription
                  </div>
                  <div className="radio-desc">Uses your connected Claude account, shared text model and separate processing permission above.</div>
                </span>
              </label>
              <label className={`radio-card${engine === 'chatgpt' ? ' selected' : ''}`}>
                <input type="radio" name="ai_engine" checked={engine === 'chatgpt'} onChange={() => update({ ai_cleanup_engine: 'chatgpt' })} />
                <span><div className="radio-title">ChatGPT subscription</div>
                  <div className="radio-desc">Uses your connected ChatGPT account, shared text model and separate processing permission above.</div></span>
              </label>
              <label className={`radio-card${engine === 'groq' ? ' selected' : ''}`}>
                <input
                  type="radio"
                  name="ai_engine"
                  checked={engine === 'groq'}
                  onChange={() => update({ ai_cleanup_engine: 'groq' })}
                />
                <span>
                  <div className="radio-title">Cloud provider</div>
                  <div className="radio-desc">Uses the provider URL and API key in General. Speed depends on the model and connection.</div>
                </span>
              </label>
            </div>
            {engine === 'local' && <ModelLicensePanel>{(canUse) => <div className="field-hint">
              {localModel?.downloaded ? <p>{canUse ? 'Model downloaded. Local cleanup can use it.' : 'Model downloaded. Accept the model terms before local cleanup can use it.'}</p>
                : localModel && <>
                  <p>Needs a one-time model download ({localModel.size_mb} MB).</p>
                  <button className="btn" type="button" onClick={downloadLocalModel} disabled={downloading || !!localModel.download?.downloading || !canUse}>
                    {downloading || localModel.download?.downloading ? 'Downloading…' : localModel.download?.paused || !!localModel.download?.downloaded_bytes ? 'Resume cleanup download' : 'Download model'}
                  </button>
                </>}
              {localModel?.name && <ModelDownloadProgress name={localModel.name} label="Gemma cleanup model" download={localModel.download} />}
              {downloadError && <span className="badge warn" role="alert">{downloadError}</span>}
            </div>}</ModelLicensePanel>}
            {(engine === 'claude' || engine === 'chatgpt') && <p className="field-hint">Connect the account, choose its text model and allow text processing above. If cleanup fails, the original transcript is kept. No paid API fallback is used.</p>}

          </div>

          {engine === 'groq' && <div className="field">
            <label className="field-label" htmlFor="cleanup-model">Cleanup model</label>
            <input id="cleanup-model" className="input" list="cleanup-models" spellCheck={false}
              value={config.cloud_cleanup_model}
              onChange={(e) => update({ cloud_cleanup_model: e.target.value })} />
            <datalist id="cleanup-models">
              <option value="openai/gpt-oss-120b">GPT-OSS 120B · recommended</option>
              <option value="openai/gpt-oss-20b">GPT-OSS 20B · faster</option>
            </datalist>
            <p className="field-hint">Current Groq choices, or enter a chat model ID supported by your provider. Local transcription can stay enabled.</p>
          </div>}

          <div className="field">
            <label className="field-label" htmlFor="cleanup-tone">Tone</label>
            <select
              id="cleanup-tone"
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

      <h2 className="settings-panel-title" style={{ marginTop: 32 }}>App profiles</h2>
      <p className="settings-panel-sub">
        Per-app overrides. When you dictate into a matching app, these replace the settings above. The selected cleanup engine and its processing permission apply even when global cleanup is off.
      </p>
      <HelpTip title="app profiles"
        why="Use a different tone, language or insertion method in particular apps."
        setup="Add a profile, enter part of the app name or choose Detect and focus that app within three seconds."
        privacy="Profiles are stored on this device; enabling cleanup for an app uses your selected cleanup engine and the same external-processing permission." />

      {profiles.map((p, i) => (
        <div className="model-card" key={i} role="group" aria-label={`App profile ${i + 1}`}>
          <div className="field">
            <label className="field-label" htmlFor={`profile-${i}-app`}>App name contains</label>
            <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
              <input
                id={`profile-${i}-app`}
                className="input"
                type="text"
                placeholder="e.g. code, slack, chrome"
                value={p.app_match}
                onChange={(e) => patchProfile(i, { app_match: e.target.value })}
              />
              <button className="btn" type="button" aria-label={`Detect app for profile ${i + 1}`} onClick={() => captureApp(i)}>Detect</button>
              <button className="btn" type="button" aria-label={`Remove app profile ${p.app_match || i + 1}`} onClick={() => setProfiles(profilesRef.current.filter((_, idx) => idx !== i))}>
                Remove
              </button>
            </div>
          </div>

          <div className="field">
            <label className="field-label" htmlFor={`profile-${i}-cleanup`}>AI cleanup</label>
            <select
              id={`profile-${i}-cleanup`}
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
            <label className="field-label" htmlFor={`profile-${i}-tone`}>Tone</label>
            <select
              id={`profile-${i}-tone`}
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
            <label className="field-label" htmlFor={`profile-${i}-language`}>Language</label>
            <select
              id={`profile-${i}-language`}
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
            <label className="field-label" htmlFor={`profile-${i}-insertion`}>Text insertion</label>
            <select
              id={`profile-${i}-insertion`}
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

      <button className="btn" type="button" onClick={() => setProfiles([...profilesRef.current, emptyProfile()])}>
        + Add app profile
      </button>
      <div className="field-hint" role="status">{captureHint}</div>
    </div>
  );
}
