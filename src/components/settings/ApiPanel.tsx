import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { HelpTip } from '../HelpTip';

interface ApiCall {
  timestamp_ms: number;
  origin: string | null;
  duration_ms: number | null;
  result_chars: number | null;
  error: string | null;
}

export interface ApiStatus {
  enabled: boolean;
  running: boolean;
  busy: boolean;
  port: number;
  allowed_origins: string[];
  error: string | null;
  calls: ApiCall[];
}

interface ApiDraft { enabled: boolean; port: string; origins: string }

const draftFrom = (status: ApiStatus): ApiDraft => ({
  enabled: status.enabled, port: String(status.port), origins: status.allowed_origins.join('\n'),
});

const differs = (draft: ApiDraft, status: ApiStatus) => {
  const saved = draftFrom(status);
  return draft.enabled !== saved.enabled || draft.port !== saved.port || draft.origins !== saved.origins;
};

function parseDraft(draft: ApiDraft) {
  if (!/^\d+$/.test(draft.port.trim()) || Number(draft.port) < 1 || Number(draft.port) > 65535) {
    throw new Error('Enter a port from 1 to 65535.');
  }
  const origins = [...new Set(draft.origins.split(/\r?\n/).map((value) => value.trim()).filter(Boolean))];
  for (const origin of origins) {
    let valid = false;
    try {
      const url = new URL(origin);
      valid = !origin.includes('*') && ['https:', 'http:'].includes(url.protocol) && url.origin === origin;
    } catch { /* The validation message below also covers malformed URLs. */ }
    if (!valid) throw new Error(`Enter an exact origin without a path, trailing slash or wildcard: ${origin}`);
  }
  return { enabled: draft.enabled, port: Number(draft.port), allowed_origins: origins };
}

/** API settings have an explicit save boundary, separate from general autosave.
 * Tokens are fetched only for a person's Reveal or Copy action. */
export function ApiPanel() {
  const [status, setStatus] = useState<ApiStatus | null>(null);
  const [draft, setDraft] = useState<ApiDraft | null>(null);
  const [pending, setPending] = useState<'save' | 'reveal' | 'copy' | 'regenerate' | null>(null);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [token, setToken] = useState<string | null>(null);
  const [confirmRegenerate, setConfirmRegenerate] = useState(false);
  const [reload, setReload] = useState(0);
  const statusRef = useRef<ApiStatus | null>(null);
  const draftRef = useRef<ApiDraft | null>(null);
  const mounted = useRef(false);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const regenerateRef = useRef<HTMLButtonElement>(null);

  const acceptStatus = (next: ApiStatus, resetDraft = false) => {
    const dirty = statusRef.current && draftRef.current && differs(draftRef.current, statusRef.current);
    statusRef.current = next;
    setStatus(next);
    if (resetDraft || !draftRef.current || !dirty) {
      const nextDraft = draftFrom(next);
      draftRef.current = nextDraft;
      setDraft(nextDraft);
    }
    if (!next.enabled) { setToken(null); setConfirmRegenerate(false); }
  };

  useEffect(() => {
    mounted.current = true;
    let active = true;
    let eventRevision = 0;
    setError('');
    const unlisten = listen<ApiStatus>('api-activity', ({ payload }) => {
      if (!active) return;
      eventRevision += 1;
      acceptStatus(payload);
    });
    invoke<ApiStatus>('get_api_status').then((next) => {
      if (active && eventRevision === 0) acceptStatus(next);
    }).catch(() => { if (active && eventRevision === 0) setError('Could not load local API status. Try again.'); });
    return () => {
      active = false;
      mounted.current = false;
      unlisten.then((stop) => stop());
    };
  }, [reload]);

  useEffect(() => { if (confirmRegenerate) cancelRef.current?.focus(); }, [confirmRegenerate]);

  const edit = (patch: Partial<ApiDraft>) => {
    if (!draftRef.current) return;
    const next = { ...draftRef.current, ...patch };
    draftRef.current = next;
    setDraft(next);
    setMessage('');
    setError('');
  };

  const save = async () => {
    if (!draftRef.current) return;
    setError('');
    setMessage('');
    let settings;
    try { settings = parseDraft(draftRef.current); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); return; }
    setPending('save');
    try {
      const next = await invoke<ApiStatus>('save_api_settings', { settings });
      if (!mounted.current) return;
      acceptStatus(next, true);
      setToken(null);
      setMessage(next.error ? 'Settings saved. The API needs attention below.' : 'API settings saved.');
    } catch (e) {
      if (mounted.current) setError(`Could not apply API settings: ${String(e)}`);
    } finally { if (mounted.current) setPending(null); }
  };

  const tokenAction = async (action: 'reveal' | 'copy' | 'regenerate') => {
    setPending(action);
    setError('');
    setMessage('');
    try {
      const value = await invoke<string>(action === 'regenerate' ? 'regenerate_api_token' : 'get_api_token');
      if (!mounted.current) return;
      if (!value) throw new Error('Enable and save the API once to create a token.');
      if (action === 'reveal') setToken(value);
      else if (action === 'copy') {
        await invoke('copy_to_clipboard', { text: value });
        if (mounted.current) setMessage('API token copied. Paste it into the app you want to connect.');
      } else {
        setToken(null);
        setConfirmRegenerate(false);
        setMessage('Token regenerated. The previous token no longer works. Copy the new token into every connected app.');
        requestAnimationFrame(() => regenerateRef.current?.focus());
      }
    } catch (e) {
      if (mounted.current) setError(`Token action failed: ${String(e)}`);
    } finally { if (mounted.current) setPending(null); }
  };

  if (!status || !draft) return <div>
    <h2 className="settings-panel-title">Local API</h2>
    {error ? <div className="inline-error" role="alert"><span>{error}</span>
      <button type="button" className="btn" onClick={() => setReload((value) => value + 1)}>Try again</button>
    </div> : <p role="status">Loading API settings…</p>}
  </div>;

  const dirty = differs(draft, status);
  const busy = pending !== null;
  const endpoint = `http://127.0.0.1:${status.port}/v1/transcribe`;
  const stateLabel = status.busy && !status.enabled ? 'Finishing current API request'
    : !status.enabled ? 'Off' : status.error ? 'Needs attention'
    : !status.running ? 'Not running' : status.busy ? 'Transcribing uploaded audio' : 'Server running on this computer';

  return <div className="api-panel">
    <h2 className="settings-panel-title">Local API</h2>
    <p className="settings-panel-sub">Let another app send audio to Lectus and receive a transcript using your local model.</p>
    <HelpTip title="the local API"
      why="Let an app transcribe audio it already has, without simulating keystrokes."
      setup="Enable and save the API, allow the app’s exact browser origin, then copy the token into that app."
      privacy="This upload-only route uses local processing and cannot open the microphone, use cloud inference, insert text or add transcripts to your dictation history." />
    <div className="api-live-status" role="status">
      <span className={`badge${status.running && !status.error ? ' ok' : ''}`}>{stateLabel}</span>
      <span>Applied settings{dirty ? ' · unsaved changes below' : ''}</span>
    </div>
    {status.error && <p className="inline-error" role="alert">{status.error}</p>}

    <div className="card">
      <div className="row">
        <div className="row-label" id="api-enable-label"><b>Enable local API</b><span>Off by default. Changes apply when you save.</span></div>
        <button type="button" className={`toggle${draft.enabled ? '' : ' off'}`} role="switch"
          aria-labelledby="api-enable-label" aria-checked={draft.enabled} disabled={busy}
          onClick={() => edit({ enabled: !draft.enabled })} />
      </div>
      <div className="row">
        <label className="row-label" htmlFor="api-port"><b>Port</b><span>Only accessible at 127.0.0.1 on this computer.</span></label>
        <input id="api-port" className="input" inputMode="numeric" type="number" min={1} max={65535}
          value={draft.port} disabled={busy} onChange={(e) => edit({ port: e.target.value })} />
      </div>
    </div>
    <div className="field api-origins">
      <label className="field-label" htmlFor="api-origins">Allowed browser origins</label>
      <textarea id="api-origins" className="textarea" rows={4} spellCheck={false} value={draft.origins}
        aria-describedby="api-origins-help" disabled={busy} onChange={(e) => edit({ origins: e.target.value })} />
      <p id="api-origins-help" className="field-hint">One exact origin per line, including scheme and port when needed. No paths, trailing slashes or wildcards. An empty list blocks browser calls; local clients still need the token.</p>
    </div>
    <div className="api-actions">
      <button type="button" className="btn btn-primary" disabled={busy || (!dirty && !status.error)} onClick={save}>
        {pending === 'save' ? 'Applying…' : 'Save API settings'}
      </button>
      {dirty && <button type="button" className="btn" disabled={busy} onClick={() => {
        acceptStatus(status, true); setError(''); setMessage('Changes discarded.');
      }}>Discard changes</button>}
    </div>

    <section className="api-section" aria-labelledby="api-token-heading">
      <h3 id="api-token-heading">Connect an app</h3>
      <p className="field-hint">Enable and save once to create a token. Every request needs it, including health checks. Only copy it into apps you choose to connect.</p>
      <label className="field-label" htmlFor="api-token">API token</label>
      <input id="api-token" className="input api-token" readOnly type={token ? 'text' : 'password'}
        value={token || ''} placeholder="Hidden" autoComplete="off" spellCheck={false} />
      <div className="api-actions">
        <button type="button" className="btn" disabled={busy || !status.enabled} onClick={() => token ? setToken(null) : void tokenAction('reveal')}>
          {pending === 'reveal' ? 'Revealing…' : token ? 'Hide token' : 'Reveal token'}
        </button>
        <button type="button" className="btn" disabled={busy || !status.enabled} onClick={() => tokenAction('copy')}>
          {pending === 'copy' ? 'Copying…' : 'Copy token'}
        </button>
        <button type="button" className="btn btn-ghost" ref={regenerateRef} disabled={busy || !status.enabled || confirmRegenerate}
          onClick={() => setConfirmRegenerate(true)}>Regenerate token</button>
      </div>
      {confirmRegenerate && <div className="history-confirm" role="group" aria-label="Confirm API token regeneration">
        <p>Replace the token? Connected apps will stop working until you give them the new token.</p>
        <div className="api-actions">
          <button type="button" className="btn" ref={cancelRef} disabled={busy} onClick={() => {
            setConfirmRegenerate(false); requestAnimationFrame(() => regenerateRef.current?.focus());
          }}>Cancel</button>
          <button type="button" className="btn btn-ghost" disabled={busy} onClick={() => tokenAction('regenerate')}>
            {pending === 'regenerate' ? 'Replacing…' : 'Replace token'}
          </button>
        </div>
      </div>}
      <p className="field-hint">Endpoint from applied settings:</p>
      <code className="api-endpoint">{endpoint}</code>
      <p className="field-hint">Send raw 16 kHz mono WAV audio, up to 8 MB and 120 seconds, with <code>Content-Type: audio/wav</code> and <code>Authorization: Bearer &lt;your token&gt;</code>.</p>
      <p className="field-hint">The API uses your local model and membership. It cannot open your microphone, paste into apps or use cloud transcription.</p>
    </section>

    {error && <p className="inline-error" role="alert">{error}</p>}
    <p className="api-feedback" role="status">{message}</p>
    <section className="api-section" aria-labelledby="api-calls-heading">
      <h3 id="api-calls-heading">Recent calls</h3>
      <p className="field-hint">Up to five recent requests. Transcript text is not shown or added to dictation history.</p>
      {status.calls.length === 0 ? <p className="history-empty">No API calls yet.</p> : <ol className="api-calls">
        {status.calls.slice(-5).reverse().map((call, index) => <li key={`${call.timestamp_ms}-${index}`}>
          <div className="api-call-heading"><time dateTime={new Date(call.timestamp_ms).toISOString()}>{new Date(call.timestamp_ms).toLocaleString()}</time>
            <span className={call.error ? 'status-err' : ''}>{call.error ? 'Failed' : 'Completed'}</span></div>
          <div className="api-call-origin">{call.origin || 'No Origin header'}</div>
          <div className="field-hint">{call.duration_ms !== null && <>Audio length: {(call.duration_ms / 1000).toFixed(1)} s · </>}
            {call.result_chars !== null ? `${call.result_chars.toLocaleString()} characters` : 'No transcript result'}</div>
          {call.error && <p className="inline-error">{call.error}</p>}
        </li>)}
      </ol>}
    </section>
  </div>;
}
