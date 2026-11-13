import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getVersion } from '@tauri-apps/api/app';
import type { PanelProps } from './types';
import { BrandMark } from '../BrandMark';
import { HelpTip } from '../HelpTip';
import { ModelDownloadProgress, type DownloadState } from './ModelDownloadProgress';
import { ModelResourceGuide } from './ModelResourceGuide';

interface ModelStatus {
  name: string;
  display_name: string;
  size_mb: number;
  multilingual: boolean;
  description: string;
  downloaded: boolean;
  active: boolean;
  loading: boolean;
  download: DownloadState;
}

export function ModelsPanel({ config }: PanelProps) {
  const [models, setModels] = useState<ModelStatus[]>([]);
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState('');
  const [version, setVersion] = useState('');
  const [updateStatus, setUpdateStatus] = useState('');
  const [updateAvailable, setUpdateAvailable] = useState('');
  const [lastUpdateError, setLastUpdateError] = useState('');

  const refresh = useCallback(() =>
    invoke<ModelStatus[]>('get_models_status').then(setModels).catch((e) => setError(String(e))), []);

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
    // Last background-check failure, persisted Rust-side, so a silently-failing updater is visible here.
    invoke<string | null>('last_update_error')
      .then((e) => { if (e) setLastUpdateError(e); })
      .catch(() => {});
    refresh();
    const unlistens = [
      listen('model-download-state', refresh),
      listen('model-ready', refresh),
      listen('model-loading', refresh),
      listen('model-active', refresh),
      listen<string>('model-load-failed', (e) => { setError(e.payload); refresh(); }),
    ];
    // Retained backend state restores progress when this panel is reopened and
    // covers completion events emitted before listener registration.
    const timer = window.setInterval(refresh, 1500);
    return () => { window.clearInterval(timer); unlistens.forEach((u) => u.then((f) => f())); };
  }, [refresh]);

  const checkUpdates = async () => {
    setUpdateStatus('Checking…');
    try {
      const info = await invoke<{ version: string } | null>('check_for_updates');
      setUpdateStatus(info ? `Update to v${info.version} downloading…` : 'Up to date');
      if (info) setUpdateAvailable(info.version);
    } catch (e) {
      setUpdateStatus(`Check failed: ${e}`);
    }
  };

  const runModelAction = async (command: 'download_model' | 'select_model', name: string) => {
    setPending(name);
    setError('');
    try {
      await invoke(command, { modelName: name });
    } catch (e) {
      if (command === 'download_model') {
        try {
          const current = await invoke<ModelStatus[]>('get_models_status');
          setModels(current);
          const download = current.find(model => model.name === name)?.download;
          // The original request finishes with a cancellation result. A
          // deliberate pause is retained progress, not a failed download.
          if (!download?.paused || download.error) setError(String(e));
        } catch { setError('Could not check saved download progress. Reopen Models to retry.'); }
      } else { setError(String(e)); }
    }
    finally { await refresh(); setPending(null); }
  };

  const busy = pending !== null || models.some((m) => m.loading || m.download.downloading);

  return (
    <div>
      <h2 className="settings-panel-title">Models</h2>
      <p className="settings-panel-sub">
        Download once, then transcribe on this computer. Smaller models use less memory;
        larger models can improve recognition but need more resources.
      </p>
      <HelpTip title="choosing a speech model"
        why="Balance recognition quality against memory use and processing time on your computer."
        setup="Download a multilingual model, select Use this model and compare a short recording in your spoken language."
        privacy="Downloads use the internet; local recognition keeps audio on this device, while cleanup and translation have their own data destinations." />
      {config.use_cloud && <p className="field-hint">Cloud transcription is enabled. These models are used when you switch back to local transcription.</p>}
      <ModelResourceGuide />
      {error && <div className="banner" role="alert">{error}</div>}
      {models.length === 0 && !error && <p role="status">Loading available models…</p>}

      <div className="model-list">
        {models.map((m) => (
          <div
            key={m.name}
            className={`model-card${m.active ? ' model-card-active' : ''}`}
          >
            <div className="model-card-header">
              <span className="model-card-name">{m.display_name}</span>
              <span className="model-card-size">{m.size_mb} MB</span>
              {m.multilingual && <span className="badge ok model-card-lang">Multilingual</span>}
              {m.active && <span className="badge ok">{config.use_cloud ? 'Local model ready' : 'Active'}</span>}
            </div>
            <p className="model-card-desc">{m.description}</p>

            <ModelDownloadProgress name={m.name} label={m.display_name} download={m.download} />

            <div className="model-card-actions">
              {!m.downloaded && !m.download.downloading && (
                <button className="btn btn-secondary" disabled={busy} onClick={() => runModelAction('download_model', m.name)}>
                  {pending === m.name ? 'Starting…' : m.download.paused || !!m.download.downloaded_bytes ? 'Resume download' : m.download.error ? 'Retry download' : 'Download'}
                </button>
              )}
              {m.downloaded && !m.active && !m.loading && (
                <button className="btn btn-primary" disabled={busy} onClick={() => runModelAction('select_model', m.name)}>
                  Use this model
                </button>
              )}
              {m.loading && (
                <button className="btn btn-secondary" disabled>
                  Loading…
                </button>
              )}
              {m.active && (
                <span className="field-hint">{config.use_cloud ? 'Ready for offline use' : 'Ready to transcribe'}</span>
              )}
            </div>
          </div>
        ))}
      </div>

      <div className="about-card">
        <BrandMark className="about-orb" />
        <div>
          <span className="about-wordmark">Lectus</span>{' '}
          <span className="about-version">v{version}</span>
          <p className="field-hint" style={{ maxWidth: 420, marginTop: 6 }}>
            Hold or tap to talk; Lectus transcribes locally or in the cloud and pastes wherever you
            point. Named after the Eclectus parrot — colourful and a great talker.
          </p>
          <div className="about-updates">
            <button className="btn btn-secondary" onClick={checkUpdates} disabled={updateStatus === 'Checking…'}>
              Check for updates
            </button>
            {updateStatus && <span className="field-hint" style={{ marginLeft: 10 }}>{updateStatus}</span>}
            {updateAvailable && (
              <span className="field-hint" style={{ marginLeft: 10 }}>
                v{updateAvailable} downloaded — restart from the banner above.
              </span>
            )}
          </div>
          {lastUpdateError && !updateStatus && (
            <div className="field-hint" style={{ marginTop: 6 }}>Last background check: {lastUpdateError}</div>
          )}
        </div>
      </div>
    </div>
  );
}
