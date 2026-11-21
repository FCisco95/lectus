import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { HelpTip } from '../HelpTip';

type Role = 'main' | 'receiver';
type Edge = 'left' | 'right' | 'top' | 'bottom';
interface LinkView {
  enabled: boolean;
  role: Role;
  side: Edge;
  roles: { role: Role; supported: boolean }[];
  this_name: string;
  peer_name: string | null;
  connected: boolean;
  status: string;
  error: boolean;
  pairing_code: string | null;
  nearby: { id: string; name: string; pairing: boolean }[];
}

const ROLE_TEXT: Record<Role, { title: string; desc: string; soon: string }> = {
  main: {
    title: 'Main computer',
    desc: 'Its keyboard, mouse and microphone work on both computers.',
    soon: 'Windows only in this version.',
  },
  receiver: {
    title: 'Other computer',
    desc: 'Controlled from the main computer. Dictation arrives here when the mouse is here.',
    soon: 'Mac only in this version.',
  },
};

const SIDES: { edge: Edge; label: string; area: string }[] = [
  { edge: 'top', label: 'Above', area: '1 / 2' },
  { edge: 'left', label: 'Left', area: '2 / 1' },
  { edge: 'right', label: 'Right', area: '2 / 3' },
  { edge: 'bottom', label: 'Below', area: '3 / 2' },
];

/** Lectus Link: one keyboard, mouse and microphone across two computers. */
export function LinkPanel() {
  const [view, setView] = useState<LinkView | null>(null);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState('');
  const [pairingId, setPairingId] = useState<string | null>(null);
  const [code, setCode] = useState('');

  const refresh = useCallback(() => {
    invoke<LinkView>('link_get').then(setView).catch((e) => setProblem(String(e)));
  }, []);

  useEffect(() => {
    let mounted = true;
    refresh();
    const unlisten = listen('link-changed', () => { if (mounted) refresh(); });
    return () => { mounted = false; unlisten.then((off) => off()); };
  }, [refresh]);

  const run = async (command: string, args?: Record<string, unknown>) => {
    setBusy(true); setProblem('');
    try { setView(await invoke<LinkView>(command, args)); return true; }
    catch (e) { setProblem(String(e)); refresh(); return false; }
    finally { setBusy(false); }
  };

  if (!view) return null;
  const save = (patch: Partial<Pick<LinkView, 'enabled' | 'role' | 'side'>>) =>
    run('link_set', { enabled: view.enabled, role: view.role, side: view.side, ...patch });
  const pair = async () => {
    if (pairingId && await run('link_pair', { id: pairingId, code })) { setPairingId(null); setCode(''); }
  };
  const isMain = view.role === 'main';
  const supported = view.roles.find((r) => r.role === view.role)?.supported ?? false;

  return (
    <div>
      <h2 className="settings-panel-title">Lectus Link</h2>
      <p className="settings-panel-sub">One keyboard, mouse and microphone for two computers on the same network.</p>
      <HelpTip title="Lectus Link"
        why="Move the mouse past the edge of this screen and keep working on the other computer. Dictation goes to the computer the mouse is on."
        setup="Turn Link on in both computers. On the other computer, note the 6-digit code. On the main computer, pick it from the list and type the code. That is the only setup."
        privacy="Everything stays on your local network and is encrypted with a key made during pairing. Only finished dictation text is sent, never audio." />

      <label className="checkbox-row">
        <input type="checkbox" checked={view.enabled} disabled={busy} onChange={(e) => save({ enabled: e.target.checked })} />
        <span className="checkbox-text">
          Turn on Link
          <small>Stop Input Leap or similar apps first; only one can control the mouse.</small>
        </span>
      </label>

      <div className="field">
        <div className="field-label" id="link-role-label">This computer is the</div>
        <div className="radio-group" role="radiogroup" aria-labelledby="link-role-label">
          {view.roles.map(({ role, supported: ok }) => (
            <label key={role} className={`radio-card${view.role === role ? ' selected' : ''}`}
              style={ok ? undefined : { opacity: 0.55 }}>
              <input type="radio" name="link_role" checked={view.role === role} disabled={busy || !ok}
                onChange={() => save({ role })} />
              <span>
                <div className="radio-title">{ROLE_TEXT[role].title}</div>
                <div className="radio-desc">{ok ? ROLE_TEXT[role].desc : ROLE_TEXT[role].soon}</div>
              </span>
            </label>
          ))}
        </div>
      </div>

      {view.enabled && supported && isMain && (
        <div className="field">
          <div className="field-label">Where is your other computer?</div>
          <div style={{ display: 'grid', gridTemplateColumns: '96px 120px 96px', gridTemplateRows: '40px 64px 40px', gap: 8, margin: '8px 0' }}>
            {SIDES.map(({ edge, label, area }) => (
              <button key={edge} className={`btn${view.side === edge ? ' btn-primary' : ''}`} disabled={busy}
                style={{ gridArea: area }} aria-pressed={view.side === edge} onClick={() => save({ side: edge })}>
                {label}
              </button>
            ))}
            <div style={{ gridArea: '2 / 2', display: 'flex', alignItems: 'center', justifyContent: 'center', textAlign: 'center',
              border: '1px solid var(--accent)', borderRadius: 8, fontSize: 12, padding: 4 }}>
              {view.this_name}
            </div>
          </div>
        </div>
      )}

      {view.enabled && supported && (
        <div className="card">
          {view.peer_name ? (
            <div className="row">
              <div className="row-label">
                <b>{view.connected ? `Connected to ${view.peer_name}` : `Paired with ${view.peer_name}`}</b>
                <span role={view.error ? 'alert' : 'status'} style={{ color: view.error ? 'var(--danger, #c44)' : undefined }}>{view.status}</span>
              </div>
              <div style={{ display: 'flex', gap: 8 }}>
                {!isMain && <button className="btn" disabled={busy} onClick={() => run('link_new_code')}>Pair another</button>}
                <button className="btn" disabled={busy} onClick={() => run('link_forget')}>Forget</button>
              </div>
            </div>
          ) : (
            <div className="row">
              <div className="row-label">
                <b>Not paired yet</b>
                <span role={view.error ? 'alert' : 'status'} style={{ color: view.error ? 'var(--danger, #c44)' : undefined }}>{view.status}</span>
              </div>
            </div>
          )}

          {!isMain && view.pairing_code && (
            <div className="row">
              <div className="row-label">
                <b>Pairing code</b>
                <span>Type it on your main computer. It changes after three wrong tries.</span>
              </div>
              <div style={{ fontSize: 28, fontWeight: 600, letterSpacing: 6, fontVariantNumeric: 'tabular-nums' }}>
                {view.pairing_code.slice(0, 3)} {view.pairing_code.slice(3)}
              </div>
            </div>
          )}

          {isMain && !view.peer_name && (
            view.nearby.length === 0 ? (
              <div className="row"><div className="row-label">
                <span>Looking for computers… Turn Link on in the other computer, on the same network.</span>
              </div></div>
            ) : view.nearby.map((c) => (
              <div className="row" key={c.id}>
                <div className="row-label">
                  <b>{c.name}</b>
                  <span>{c.pairing ? 'Ready to pair' : 'Paired with another computer. Choose Pair another there first.'}</span>
                </div>
                {pairingId === c.id ? (
                  <div style={{ display: 'flex', gap: 8 }}>
                    <input className="input" aria-label="Pairing code" inputMode="numeric" autoComplete="off" autoFocus
                      placeholder="6-digit code" value={code} maxLength={7} style={{ width: 120 }}
                      onChange={(e) => setCode(e.target.value)} onKeyDown={(e) => { if (e.key === 'Enter') pair(); }} />
                    <button className="btn btn-primary" disabled={busy || code.replace(/\D/g, '').length !== 6} onClick={pair}>
                      {busy ? 'Pairing…' : 'Pair'}
                    </button>
                  </div>
                ) : (
                  <button className="btn btn-primary" disabled={busy || !c.pairing}
                    onClick={() => { setPairingId(c.id); setCode(''); setProblem(''); }}>Pair</button>
                )}
              </div>
            ))
          )}

          {problem && (
            <div className="row"><div className="row-label">
              <span role="alert" style={{ color: 'var(--danger, #c44)' }}>{problem}</span>
            </div></div>
          )}
        </div>
      )}

      {view.enabled && supported && isMain && view.peer_name && (
        <p className="settings-panel-sub" style={{ marginTop: 12 }}>
          Push the mouse past the {SIDES.find((s) => s.edge === view.side)?.label.toLowerCase()} edge to use {view.peer_name}.
          Scroll Lock brings the mouse and keyboard back. Right Ctrl keeps dictating, on whichever computer the mouse is.
        </p>
      )}
    </div>
  );
}
