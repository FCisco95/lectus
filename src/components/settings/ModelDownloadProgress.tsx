import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface DownloadState {
  downloading: boolean;
  progress: number;
  error: string | null;
  paused?: boolean;
  downloaded_bytes?: number;
  total_bytes?: number | null;
  message?: string | null;
  can_pause?: boolean;
}

export function ModelDownloadProgress({ name, label, download }: {
  name: string; label: string; download?: DownloadState;
}) {
  const [pausing, setPausing] = useState(false);
  const [pauseError, setPauseError] = useState('');
  useEffect(() => { if (!download?.downloading) setPausing(false); }, [download?.downloading]);
  if (!download) return null;
  const percent = Math.round(Math.max(0, Math.min(1, Number.isFinite(download.progress) ? download.progress : 0)) * 100);
  const pause = async () => {
    setPausing(true); setPauseError('');
    try { await invoke('cancel_model_download', { modelName: name }); }
    catch (error) { setPauseError(String(error)); setPausing(false); }
  };
  return <div className="model-download" aria-busy={download.downloading}>
    {(download.downloading || download.paused || !!download.downloaded_bytes) && <>
      <div className="progress-track" role="progressbar" aria-label={`Downloading ${label}`}
        aria-valuemin={0} aria-valuemax={100} aria-valuenow={download.total_bytes === null ? undefined : percent}>
        <div className="progress-fill" style={{ width: `${percent}%` }} />
      </div>
      <p className="field-hint" role="status">
        {download.message || (download.paused ? 'Paused. Downloaded data is kept; resume when ready.' : 'Downloading…')}
        {download.total_bytes !== null && ` · ${percent}%`}
        {!!download.downloaded_bytes && ` · ${(download.downloaded_bytes / 1_000_000).toFixed(1)} MB saved`}
      </p>
    </>}
    {download.downloading && download.can_pause !== false && <button type="button" className="btn btn-secondary" onClick={pause} disabled={pausing}>
      {pausing ? 'Pausing…' : 'Pause download'}
    </button>}
    {download.error && <p className="field-hint" role="alert">{download.error}</p>}
    {pauseError && <p className="field-hint" role="alert">{pauseError}</p>}
  </div>;
}
