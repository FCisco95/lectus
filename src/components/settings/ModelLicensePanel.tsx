import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface ModelLicenseStatus {
  model_id: string;
  title: string;
  version: string;
  fingerprint: string;
  accepted: boolean;
  terms: string;
  policy: string;
}

/** Model licensing is independent of Lectus terms and external-processing choices. */
export function ModelLicensePanel({ children }: { children?: (canUse: boolean) => ReactNode }) {
  const [status, setStatus] = useState<ModelLicenseStatus | null>(null);
  const [checked, setChecked] = useState(false);
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState<'accept' | 'revoke' | null>(null);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [reload, setReload] = useState(0);
  const mounted = useRef(false);
  const eventRevision = useRef(0);
  const latestStatus = useRef<ModelLicenseStatus | null>(null);
  const statusRef = useRef<HTMLParagraphElement>(null);
  const choiceRef = useRef<HTMLInputElement>(null);
  const headingId = useId();
  const termsId = useId();
  const policyId = useId();

  useEffect(() => {
    mounted.current = true;
    let active = true;
    let eventReceived = false;
    setLoading(true);
    setError('');
    setMessage('');
    const unlisten = listen<ModelLicenseStatus>('cleanup-license-changed', ({ payload }) => {
      if (!active) return;
      eventReceived = true;
      eventRevision.current += 1;
      latestStatus.current = payload;
      setStatus(payload);
      setChecked(false);
      setLoading(false);
      setMessage(payload.accepted ? '' : 'Model agreement status updated.');
    });
    invoke<ModelLicenseStatus>('get_cleanup_model_license').then((value) => {
      if (active && !eventReceived) { latestStatus.current = value; setStatus(value); setChecked(false); }
    }).catch(() => {
      if (active && !eventReceived) { setStatus(null); setError('Could not load the bundled Gemma terms. Local cleanup remains unavailable. Try again.'); }
    }).finally(() => { if (active) setLoading(false); });
    return () => { active = false; mounted.current = false; unlisten.then((stop) => stop()); };
  }, [reload]);

  const act = async (action: 'accept' | 'revoke') => {
    if (!status || pending || loading || (action === 'accept' && !checked)) return;
    setPending(action); setError(''); setMessage('');
    const startedRevision = eventRevision.current;
    try {
      const result = action === 'accept'
        ? await invoke<ModelLicenseStatus>('accept_cleanup_model_license', { fingerprint: status.fingerprint })
        : await invoke<ModelLicenseStatus>('revoke_cleanup_model_license');
      if (!mounted.current) return;
      // A newer event may reflect a revoke from another window after this
      // command completed; never replace it with the older command response.
      const next = eventRevision.current === startedRevision ? result : latestStatus.current ?? result;
      latestStatus.current = next;
      setStatus(next); setChecked(false);
      setMessage(next.accepted
        ? ''
        : action === 'revoke' ? 'Model acceptance revoked. Local Gemma cleanup is unavailable.'
          : 'Model terms are not accepted. Review the current documents before trying again.');
      requestAnimationFrame(() => {
        if (next.accepted) statusRef.current?.focus();
        else choiceRef.current?.focus();
      });
    } catch (reason) { if (mounted.current) setError(`Could not ${action} model terms: ${String(reason)}`); }
    finally { if (mounted.current) setPending(null); }
  };

  const canUse = !loading && !pending && status?.accepted === true;
  return <section aria-labelledby={headingId}>
    <h3 id={headingId}>Gemma model terms</h3>
    <p className="field-hint">Gemma powers local cleanup and has separate model terms and use restrictions. This agreement is separate from Lectus terms and cloud-processing permission.</p>
    <p className="field-hint">Ordinary dictation does not require this model agreement. Leave it unaccepted to keep Gemma cleanup unavailable; no cloud engine is selected automatically.</p>
    {loading && <p className="field-hint" role="status">Loading bundled model terms…</p>}
    {!loading && status && <>
      <p className="field-hint">{status.title} · version {status.version}</p>
      <details className="help-tip">
        <summary id={termsId}>Read Gemma Terms</summary>
        <div role="region" aria-labelledby={termsId} tabIndex={0} style={{ maxHeight: 280, overflowY: 'auto', padding: '8px 12px' }}>
          <pre style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere', font: 'inherit', lineHeight: 1.6 }}>{status.terms}</pre>
        </div>
      </details>
      <details className="help-tip">
        <summary id={policyId}>Read incorporated use restrictions</summary>
        <div role="region" aria-labelledby={policyId} tabIndex={0} style={{ maxHeight: 280, overflowY: 'auto', padding: '8px 12px' }}>
          <pre style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere', font: 'inherit', lineHeight: 1.6 }}>{status.policy}</pre>
        </div>
      </details>
      {status.accepted ? <>
        <p className="field-hint" role="status" ref={statusRef} tabIndex={-1}>Current Gemma model terms accepted on this device.</p>
        <button className="btn btn-ghost" type="button" disabled={pending !== null} onClick={() => act('revoke')}>
          {pending === 'revoke' ? 'Revoking…' : 'Revoke model acceptance'}
        </button>
      </> : <>
        <label className="checkbox-row">
          <input ref={choiceRef} type="checkbox" checked={checked} disabled={pending !== null} onChange={(event) => setChecked(event.target.checked)} />
          <span className="checkbox-text">I accept the Gemma Terms and incorporated use restrictions</span>
        </label>
        <button className="btn btn-primary" type="button" disabled={!checked || pending !== null} onClick={() => act('accept')}>
          {pending === 'accept' ? 'Saving acceptance…' : 'Accept model terms'}
        </button>
        <p className="field-hint">Review and accept before downloading or using Gemma, including a model already downloaded.</p>
      </>}
    </>}
    {error && <div className="inline-error" role="alert"><span>{error}</span>
      <button className="btn" type="button" disabled={loading || pending !== null} onClick={() => {
        setChecked(false); setLoading(true); setReload((value) => value + 1);
      }}>{status ? 'Reload model terms' : 'Retry model terms'}</button>
    </div>}
    <p className="field-hint" role="status">{message}</p>
    {children?.(canUse)}
  </section>;
}
