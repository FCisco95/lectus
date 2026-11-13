import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface LegalStatus { ready: boolean; accepted: boolean; terms_version: string; privacy_version: string; terms_hash: string; privacy_hash: string; terms: string; privacy: string }

export function PrivacySummary() {
  const [documents, setDocuments] = useState<LegalStatus | null>(null);
  const [error, setError] = useState('');
  const open = async () => {
    try { setDocuments(await invoke<LegalStatus>('get_legal_status')); setError(''); }
    catch { setError('Could not load the bundled documents. Try again.'); }
  };
  return <section className="privacy-summary" aria-label="Your data and choices">
    <h3>Your data and choices</h3>
    <p>After setup, Lectus keeps a rolling half-second microphone buffer in memory to catch the start of your sentence. Receive mode leaves the microphone off.</p>
    <p>Local models process on your device. Optional cloud speech sends audio; cloud cleanup or translation sends text. Each external route needs your choice. Local processing still allows update, model-download and membership checks.</p>
    <p>Dictation history and settings are stored on your device without Lectus encryption. Clear history whenever you want. API calls do not enter dictation history.</p>
    <button type="button" className="btn" onClick={open}>Read privacy and terms</button>
    <button type="button" className="btn" onClick={() => invoke('open_legal_notices').catch((e) => setError(String(e)))}>Open third-party licenses</button>
    {error && <p role="alert">{error}</p>}
    {documents && <DocumentText documents={documents} />}
    {documents?.ready && !documents.accepted && <LegalSetup onReady={(accepted) => {
      if (accepted) setDocuments((value) => value ? { ...value, accepted: true } : value);
    }} />}
  </section>;
}

function DocumentText({ documents }: { documents: LegalStatus }) {
  return <div className="legal-documents">
    {!documents.ready && <p role="status">Legal documents are being prepared. These drafts are not a final agreement.</p>}
    <details><summary>Terms · {documents.terms_version}</summary><pre style={{ whiteSpace: 'pre-wrap', font: 'inherit' }}>{documents.terms}</pre></details>
    <details><summary>Privacy Notice · {documents.privacy_version}</summary><pre style={{ whiteSpace: 'pre-wrap', font: 'inherit' }}>{documents.privacy}</pre></details>
  </div>;
}

export function LegalSetup({ onReady }: { onReady: (ready: boolean) => void }) {
  const [documents, setDocuments] = useState<LegalStatus | null>(null);
  const [checked, setChecked] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const callback = useRef(onReady); callback.current = onReady;
  useEffect(() => {
    let active = true;
    invoke<LegalStatus>('get_legal_status').then((value) => { if (active) { setDocuments(value); callback.current(value.ready && value.accepted); } })
      .catch(() => { if (active) setError('Could not load the legal documents. Reopen setup to try again.'); });
    return () => { active = false; };
  }, []);
  const accept = async () => {
    if (!documents?.ready || !checked) return;
    setPending(true); setError('');
    try {
      const result = await invoke<LegalStatus>('accept_legal_terms', { termsVersion: documents.terms_version, termsHash: documents.terms_hash });
      setDocuments(result); callback.current(result.ready && result.accepted);
    } catch (e) { setError(String(e)); callback.current(false); }
    finally { setPending(false); }
  };
  return <div>
    <p>Read the Terms and Privacy Notice before setup. Optional cloud processing is chosen separately in settings.</p>
    {documents ? <DocumentText documents={documents} /> : !error && <p role="status">Loading documents…</p>}
    {documents?.ready && !documents.accepted && <>
      <label className="checkbox-row"><input type="checkbox" checked={checked} disabled={pending} onChange={(e) => setChecked(e.target.checked)} /><span>I have read and agree to the Terms.</span></label>
      <p className="field-hint">The Privacy Notice explains data handling; this does not enable cloud processing.</p>
      <button className="btn btn-primary" type="button" onClick={accept} disabled={!checked || pending}>{pending ? 'Saving…' : 'Accept Terms'}</button>
    </>}
    {documents?.accepted && <p role="status">Current Terms accepted on this device.</p>}
    {error && <p role="alert" className="inline-error">{error}</p>}
  </div>;
}
