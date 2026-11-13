import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { PanelProps } from './types';

type Provider = 'chatgpt' | 'claude';
type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'unavailable' | 'error';
interface ConnectionStatus { provider: Provider; state: ConnectionState; account_label?: string | null; message: string }
interface SubscriptionModel { id: string; name: string }
interface TestResult { text: string; elapsed_ms: number }
const providers: Provider[] = ['chatgpt', 'claude'];
const label = (provider: Provider) => provider === 'chatgpt' ? 'ChatGPT' : 'Claude';

/** Account state is separate from processing permission and engine selection. */
export function AIConnections({ config, update }: PanelProps) {
  const [connections, setConnections] = useState<Partial<Record<Provider, ConnectionStatus>>>({});
  const [errors, setErrors] = useState<Partial<Record<Provider, string>>>({});
  const [busy, setBusy] = useState<Partial<Record<Provider, string>>>({});
  const [results, setResults] = useState<Partial<Record<Provider, string>>>({});
  const mounted = useRef(false);
  const revisions = useRef({ chatgpt: 0, claude: 0 });
  const testRevisions = useRef({ chatgpt: 0, claude: 0 });
  const reading = useRef(false);
  const configRef = useRef(config);
  configRef.current = config;

  const refresh = async (only?: Provider, force = false) => {
    const snapshot = { ...revisions.current };
    try {
      const statuses = await invoke<ConnectionStatus[]>('get_ai_connections', force ? { refresh: true } : undefined);
      if (!mounted.current) return;
      for (const provider of only ? [only] : providers) {
        if (snapshot[provider] !== revisions.current[provider]) continue;
        const status = statuses.find(value => value.provider === provider);
        if (!status) throw new Error('Missing provider state');
        setConnections(previous => ({ ...previous, [provider]: status }));
        setErrors(previous => ({ ...previous, [provider]: '' }));
      }
    } catch {
      if (!mounted.current) return;
      for (const provider of only ? [only] : providers) {
        if (snapshot[provider] === revisions.current[provider]) {
          setErrors(previous => ({ ...previous, [provider]: 'Could not check this account. Select Check connection to retry.' }));
        }
      }
    }
  };

  useEffect(() => {
    mounted.current = true;
    void refresh();
    return () => { mounted.current = false; revisions.current.chatgpt++; revisions.current.claude++; };
  }, []);

  const connecting = providers.some(provider => connections[provider]?.state === 'connecting');
  useEffect(() => {
    if (!connecting) return;
    // Only unfinished logins are polled. Connected CLI accounts are checked on demand.
    const timer = window.setInterval(async () => {
      if (reading.current) return;
      reading.current = true;
      try { await refresh(); } finally { reading.current = false; }
    }, 2500);
    return () => window.clearInterval(timer);
  }, [connecting]);

  const act = async (provider: Provider, action: 'login' | 'cancel' | 'disconnect' | 'check' | 'test') => {
    const revision = ++revisions.current[provider];
    const testRevision = ++testRevisions.current[provider];
    const current = () => mounted.current && revision === revisions.current[provider]
      && (action !== 'test' || testRevision === testRevisions.current[provider]);
    setErrors(previous => ({ ...previous, [provider]: '' }));
    setResults(previous => ({ ...previous, [provider]: '' }));
    setBusy(previous => ({ ...previous, [provider]: action }));
    if (action === 'login') setConnections(previous => ({ ...previous, [provider]: { provider, state: 'connecting', message: 'Finish sign-in in your browser.' } }));
    try {
      if (action === 'check') await refresh(provider, true);
      else if (action === 'test') {
        const model = provider === 'chatgpt' ? configRef.current.chatgpt_model : configRef.current.claude_model;
        const result = await invoke<TestResult>('test_subscription', { provider, model: model || '' });
        const latestModel = provider === 'chatgpt' ? configRef.current.chatgpt_model : configRef.current.claude_model;
        if (current() && (latestModel || '') === (model || '')) {
          setResults(previous => ({ ...previous, [provider]: `Synthetic text test passed (${Math.round(result.elapsed_ms)} ms). Your dictation flow has not been tested.` }));
        }
      } else {
        const command = action === 'login' ? 'start_ai_login' : action === 'cancel' ? 'cancel_ai_login' : 'disconnect_ai';
        await invoke(command, { provider });
        if (current()) await refresh(provider);
      }
    } catch {
      if (current()) {
        const message = action === 'test' ? 'Text test failed. Check the account, selected model and your provider usage limits, then retry.'
          : action === 'login' ? 'Sign-in could not start. Check the connection for setup details, then retry.'
          : action === 'cancel' ? 'Could not cancel sign-in. Check the connection and retry.'
          : 'Account action failed. Check the connection and retry.';
        setErrors(previous => ({ ...previous, [provider]: message }));
        if (action === 'login') setConnections(previous => ({ ...previous, [provider]: { provider, state: 'error', message: '' } }));
      }
    } finally {
      if (current()) setBusy(previous => ({ ...previous, [provider]: '' }));
    }
  };

  return <section aria-labelledby="ai-connections-title">
    <h3 id="ai-connections-title">Subscription accounts</h3>
    <p className="field-hint">Use an eligible ChatGPT or Claude subscription for text cleanup and translation. Speech recognition stays local or uses your separately billed API provider. Provider plan and usage limits apply, including any extra usage you enable with the provider. Lectus never switches to a paid API automatically.</p>
    <p className="field-hint">Sign-in does not enable cleanup or translation. A connected account does not prove text generation works; use the synthetic text test.</p>
    {providers.map(provider => {
      const status = connections[provider];
      const name = label(provider);
      const permission = provider === 'chatgpt' ? config.chatgpt_processing_acknowledged : config.claude_processing_acknowledged;
      const model = (provider === 'chatgpt' ? config.chatgpt_model : config.claude_model) || '';
      const pending = !!busy[provider];
      return <div className="model-card" role="group" aria-label={`${name} connection`} key={provider}>
        <h3>{name}</h3>
        <p role="status">{!status ? 'Checking account…' : status.state === 'connected' ? `Connected${status.account_label ? ` · ${status.account_label}` : ''} · ${results[provider] ? 'synthetic text test passed' : 'text generation not verified'}` : status.state === 'connecting' ? 'Connecting · finish sign-in in your browser' : status.state === 'unavailable' ? 'Unavailable' : status.state === 'error' ? 'Account needs attention' : 'Disconnected'}</p>
        {status?.message && <p className="field-hint">{status.message}</p>}
        {provider === 'claude' && <p className="field-hint">Requires the official Claude Code CLI. Sign-in uses a separate Lectus account directory and the official provider login.</p>}
        <div className="help-actions">
          {status?.state === 'connecting' ? <button className="btn" type="button" disabled={pending && busy[provider] !== 'login'} onClick={() => void act(provider, 'cancel')}>Cancel {name} sign-in</button>
            : status?.state === 'connected' ? <button className="btn" type="button" disabled={pending && busy[provider] !== 'test'} onClick={() => void act(provider, 'disconnect')}>Disconnect {name}</button>
              : <button className="btn" type="button" disabled={pending || !status} onClick={() => void act(provider, 'login')}>Sign in to {name}</button>}
          <button className="btn" type="button" disabled={pending} onClick={() => void act(provider, 'check')}>Check {name} connection</button>
          <button className="btn" type="button" disabled={pending || status?.state !== 'connected' || (provider === 'chatgpt' && !model)} onClick={() => void act(provider, 'test')}>{busy[provider] === 'test' ? `Testing ${name}…` : `Test ${name}`}</button>
        </div>
        <SubscriptionModelPicker provider={provider} connected={status?.state === 'connected'} value={model}
          onChange={value => {
            // Text settings invalidate test output, never a pending account action.
            testRevisions.current[provider]++;
            setBusy(previous => ({ ...previous, [provider]: previous[provider] === 'test' ? '' : previous[provider] }));
            setResults(previous => ({ ...previous, [provider]: '' }));
            update(provider === 'chatgpt' ? { chatgpt_model: value } : { claude_model: value });
          }} />
        <label className="checkbox-row">
          <input type="checkbox" checked={permission === true} onChange={event => {
            testRevisions.current[provider]++;
            setBusy(previous => ({ ...previous, [provider]: previous[provider] === 'test' ? '' : previous[provider] }));
            setResults(previous => ({ ...previous, [provider]: '' }));
            update(provider === 'chatgpt' ? { chatgpt_processing_acknowledged: event.target.checked } : { claude_processing_acknowledged: event.target.checked });
          }} />
          <span className="checkbox-text">Allow transcript text processing through {name}
            <small>Allows enabled cleanup, app profiles and translation to send text to this account. The model above is shared by cleanup and translation. Tests send synthetic text only.</small>
          </span>
        </label>
        {errors[provider] && <p className="inline-error" role="alert">{errors[provider]}</p>}
        {results[provider] && <p className="field-hint" role="status">{results[provider]}</p>}
      </div>;
    })}
  </section>;
}

function SubscriptionModelPicker({ provider, connected, value, onChange }: { provider: Provider; connected: boolean; value: string; onChange: (value: string) => void }) {
  const [models, setModels] = useState<SubscriptionModel[]>([]);
  const [error, setError] = useState(false);
  const [loading, setLoading] = useState(false);
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    let active = true;
    setModels([]); setError(false);
    if (!connected) { setLoading(false); return; }
    setLoading(true);
    invoke<SubscriptionModel[]>('list_subscription_models', { provider })
      .then(value => { if (active) setModels(value); })
      .catch(() => { if (active) setError(true); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [provider, connected, retry]);
  const id = `${provider}-subscription-model`;
  return <div className="field">
    <label className="field-label" htmlFor={id}>{label(provider)} text model</label>
    <select id={id} className="select" value={value} disabled={!connected || loading || error} onChange={event => onChange(event.target.value)}>
      <option value="">{provider === 'claude' ? 'Official default model' : 'Choose an available model'}</option>
      {value && !models.some(model => model.id === value) && <option value={value}>{value} · saved selection (availability not verified)</option>}
      {models.filter(model => model.id !== '').map(model => <option key={model.id} value={model.id}>{model.name}</option>)}
    </select>
    {!connected && <p className="field-hint">Connect this account to load its available models.</p>}
    {loading && <p className="field-hint" role="status">Loading available models…</p>}
    {connected && !loading && !error && !models.length && provider === 'chatgpt' && <p className="field-hint">No eligible models returned. Check your account access and retry.</p>}
    {connected && <button className="btn" type="button" disabled={loading} onClick={() => setRetry(previous => previous + 1)}>Refresh {label(provider)} models</button>}
    {error && <p className="inline-error" role="alert">Could not load available models. Refresh models to retry.</p>}
  </div>;
}
