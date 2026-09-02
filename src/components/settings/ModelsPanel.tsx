import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getVersion } from '@tauri-apps/api/app';
import type { PanelProps } from './types';

const SWATCHES = ['#1db584', '#17a2a2', '#3b82f6', '#e91e8c', '#ff6b5a', '#ffa500'];

interface ModelStatus {
  name: string;
  display_name: string;
  size_mb: number;
  multilingual: boolean;
  description: string;
  downloaded: boolean;
  active: boolean;
}

export function ModelsPanel({ config }: PanelProps) {
  const [models, setModels] = useState<ModelStatus[]>([]);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);
  const [loading, setLoading] = useState<string | null>(null);
  const [version, setVersion] = useState('');
  const [updateStatus, setUpdateStatus] = useState('');
  const [updateAvailable, setUpdateAvailable] = useState('');
  const [lastUpdateError, setLastUpdateError] = useState('');

  const refresh = () =>
    invoke<ModelStatus[]>('get_models_status').then(setModels).catch(console.error);

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
    // Last background-check failure, persisted Rust-side, so a silently-failing updater is visible here.
    invoke<string | null>('last_update_error')
      .then((e) => { if (e) setLastUpdateError(e); })
      .catch(() => {});
    refresh();
    const unlistens = [
      listen<number>('model-download-progress', (e) => setProgress(e.payload)),
      listen<string>('model-ready', (e) => {
        setDownloading(null);
        setProgress(0);
        refresh();
        // If this is the first time tiny downloads, auto-select it.
        if (e.payload === config.model_name) refresh();
      }),
      listen<string>('model-download-failed', () => {
        setDownloading(null);
        setProgress(0);
      }),
      listen<string>('model-loading', (e) => setLoading(e.payload)),
      listen<string>('model-active', () => {
        setLoading(null);
        refresh();
      }),
      listen<string>('model-load-failed', () => setLoading(null)),
    ];
    return () => { unlistens.forEach((u) => u.then((f) => f())); };
  }, [config.model_name]);

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

  const handleDownload = (name: string) => {
    setDownloading(name);
    setProgress(0);
    invoke('download_model', { modelName: name }).catch((e) => {
      console.error(e);
      setDownloading(null);
    });
  };

  const handleSelect = (name: string) => {
    setLoading(name);
    invoke('select_model', { modelName: name }).catch((e) => {
      console.error(e);
      setLoading(null);
    });
  };

  const pct = Math.round(progress * 100);

  return (
    <div>
      <h2 className="settings-panel-title">Models</h2>
      <p className="settings-panel-sub">
        Download any Whisper model and switch between them. Tiny is the best starting point —
        fast on CPU and supports 99 languages including Portuguese.
      </p>

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
              {m.active && <span className="badge ok">Active</span>}
            </div>
            <p className="model-card-desc">{m.description}</p>

            {downloading === m.name && (
              <div style={{ marginTop: 8 }}>
                <div className="progress-track">
                  <div className="progress-fill" style={{ width: `${pct}%` }} />
                </div>
                <div className="field-hint" style={{ marginTop: 4 }}>Downloading… {pct}%</div>
              </div>
            )}

            <div className="model-card-actions">
              {!m.downloaded && downloading !== m.name && (
                <button className="btn btn-secondary" onClick={() => handleDownload(m.name)}>
                  Download
                </button>
              )}
              {m.downloaded && !m.active && loading !== m.name && (
                <button className="btn btn-primary" onClick={() => handleSelect(m.name)}>
                  Use this model
                </button>
              )}
              {loading === m.name && (
                <button className="btn btn-secondary" disabled>
                  Loading…
                </button>
              )}
              {m.active && (
                <span className="field-hint">Currently active</span>
              )}
            </div>
          </div>
        ))}
      </div>

      <div className="about-card">
        <div className="about-orb" />
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
          <div className="swatches">
            {SWATCHES.map((c) => (
              <span className="swatch" key={c} style={{ background: c }} />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
