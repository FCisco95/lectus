import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

type Mode = 'local' | 'send' | 'receive';
interface RemoteSettings {
  mode: Mode;
  receiver_code: string;
  peer_code: string;
  status: string;
  error: boolean;
}
interface Status { status: string; error: boolean }

export function RemoteDictationPanel() {
  const [saved, setSaved] = useState<RemoteSettings | null>(null);
  const [mode, setMode] = useState<Mode>('local');
  const [code, setCode] = useState('');
  const [status, setStatus] = useState<Status>({ status: '', error: false });
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let mounted = true;
    invoke<RemoteSettings>('remote_get_settings').then((s) => {
      if (!mounted) return;
      setSaved(s); setMode(s.mode); setCode(s.peer_code);
      setStatus({ status: s.status, error: s.error });
    }).catch((e) => { if (mounted) setStatus({ status: String(e), error: true }); });
    const unlisten = listen<Status>('remote-delivery-status', (e) => {
      if (mounted) setStatus(e.payload);
    });
    return () => { mounted = false; unlisten.then((off) => off()); };
  }, []);

  const changed = saved && (mode !== saved.mode || (mode === 'send' && code.trim() !== saved.peer_code));
  const save = async () => {
    setBusy(true); setCopied(false);
    try {
      const s = await invoke<RemoteSettings>('remote_save_settings', { mode, peerCode: code });
      setSaved(s); setMode(s.mode); setCode(s.peer_code);
      setStatus({ status: s.status, error: s.error });
    } catch (e) {
      setStatus({ status: String(e), error: true });
      // Activation can fail after the choice was saved (e.g. firewall). Reflect reality.
      const s = await invoke<RemoteSettings>('remote_get_settings').catch(() => null);
      if (s) setSaved(s);
    } finally { setBusy(false); }
  };
  const test = async () => {
    setBusy(true);
    try {
      await invoke('remote_test_connection');
      setStatus({ status: 'Connected. Ready to dictate.', error: false });
    } catch (e) { setStatus({ status: String(e), error: true }); }
    finally { setBusy(false); }
  };
  const copyCode = async () => {
    if (!saved?.receiver_code) return;
    try {
      await invoke('copy_to_clipboard', { text: saved.receiver_code });
      setCopied(true);
    } catch (e) { setStatus({ status: String(e), error: true }); }
  };

  return (
    <div className="card">
      <div className="row">
        <div className="row-label">
          <b>Dictation destination</b>
          <span>Use your microphone on one computer and insert the text on another.</span>
        </div>
        <select className="select" aria-label="Dictation destination" value={mode}
          disabled={busy || !saved} onChange={(e) => { setMode(e.target.value as Mode); setCopied(false); }}>
          <option value="local">This computer</option>
          <option value="send">Send to another computer</option>
          <option value="receive">Receive from another computer</option>
        </select>
      </div>
      {mode === 'receive' && (
        <div className="row">
          <div className="row-label">
            <b>Pair your microphone computer</b>
            <span>Save this mode, then copy the code to Lectus on the computer with your microphone.
              Local recording is off while receiving. Keep this app running and focus the field where you want the text.</span>
          </div>
          {saved?.mode === 'receive' && saved.receiver_code && (
            <button className="btn" onClick={copyCode} disabled={busy}>{copied ? 'Code copied' : 'Copy connection code'}</button>
          )}
        </div>
      )}
      {mode === 'send' && (
        <>
          <div className="row">
            <div className="row-label">
              <b>Connection code</b>
              <span>On the other computer, choose Receive and copy its code here. Both computers must share a local network.</span>
            </div>
            <input className="input" aria-label="Connection code" type="password" autoComplete="off"
              spellCheck={false} placeholder="Paste connection code" value={code} disabled={busy}
              style={{ maxWidth: 260 }} onChange={(e) => setCode(e.target.value)} />
          </div>
          <div className="row">
            <div className="row-label">
              <span>Dictations go to the paired computer until you select This computer.
                Your normal shortcut still records this microphone. Only finished text is sent.</span>
            </div>
          </div>
        </>
      )}
      <div className="row">
        <div className="row-label">
          <span role={status.error ? 'alert' : 'status'} style={{ color: status.error ? 'var(--danger, #c44)' : undefined }}>
            {status.status}
          </span>
        </div>
        <div style={{ display: 'flex', gap: 8 }}>
          {saved?.mode === 'send' && !changed && (
            <button className="btn" disabled={busy} onClick={test}>Test connection</button>
          )}
          <button className="btn btn-primary" disabled={busy || !saved || (!changed && !status.error)}
            onClick={save}>{busy ? 'Working…' : 'Save destination'}</button>
        </div>
      </div>
    </div>
  );
}

/** A failed send must remain visible even when General is not the selected page. */
export function RemoteDeliveryNotice() {
  const [status, setStatus] = useState<Status | null>(null);
  useEffect(() => {
    let mounted = true;
    invoke<RemoteSettings>('remote_get_settings').then((s) => {
      if (mounted && s.error) setStatus(s);
    }).catch(() => {});
    const unlisten = listen<Status>('remote-delivery-status', (e) => {
      if (mounted) setStatus(e.payload);
    });
    return () => { mounted = false; unlisten.then((off) => off()); };
  }, []);
  if (!status?.error) return null;
  return <div className="banner" role="alert">
    <span><b>Remote dictation:</b> {status.status}</span>
    <button className="btn" onClick={() => setStatus(null)}>Dismiss</button>
  </div>;
}

