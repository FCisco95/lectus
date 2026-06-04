import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { PanelProps } from './types';

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

  const refresh = () =>
    invoke<ModelStatus[]>('get_models_status').then(setModels).catch(console.error);

  useEffect(() => {
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
    </div>
  );
}
