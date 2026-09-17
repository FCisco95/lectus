import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { LicenseFloor, LicenseStatus } from './types';
import { shortAddress } from './types';

const EXPLORER = 'https://solscan.io/account/';

function usd(value: number): string {
  return `$${value.toLocaleString(undefined, { maximumFractionDigits: 2 })}`;
}

function org(value: number): string {
  return `${Math.round(value).toLocaleString()} ORG`;
}

export function MembershipPanel() {
  const [status, setStatus] = useState<LicenseStatus | null>(null);
  const [floor, setFloor] = useState<LicenseFloor | null>(null);
  const [linking, setLinking] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    invoke<LicenseStatus>('license_status').then(setStatus).catch(() => {});
    invoke<LicenseFloor>('license_floor').then(setFloor).catch(() => {});
    // The background re-check can land while this panel is open.
    const un = listen<LicenseStatus>('license-changed', (e) => setStatus(e.payload));
    return () => { un.then((f) => f()); };
  }, []);

  const connect = async () => {
    setError('');
    setLinking(true);
    try {
      setStatus(await invoke<LicenseStatus>('link_wallet'));
    } catch (e) {
      setError(`${e}`);
    } finally {
      setLinking(false);
    }
  };

  const refresh = async () => {
    setError('');
    setBusy(true);
    try {
      setStatus(await invoke<LicenseStatus>('refresh_license'));
    } catch (e) {
      setError(`Could not reach the chain: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const unlink = async () => {
    setError('');
    setBusy(true);
    try {
      setStatus(await invoke<LicenseStatus>('unlink_wallet'));
    } catch (e) {
      setError(`${e}`);
    } finally {
      setBusy(false);
    }
  };

  const required = floor?.tokens_required ?? null;
  const pubkey = status && status.kind !== 'unlinked' ? status.pubkey : '';

  return (
    <div>
      <h2 className="settings-panel-title">Membership</h2>
      <p className="settings-panel-sub">
        Lectus is free for people who hold ORGANIC. No account, no subscription.
      </p>

      <div className="card">
        <div className="row">
          <div className="row-label">
            <b>Wallet</b>
            <span>
              {status?.kind === 'unlinked'
                ? 'Not linked yet.'
                : `Linked: ${shortAddress(pubkey)}`}
            </span>
          </div>
          {status?.kind === 'unlinked' ? (
            <button className="btn btn-primary" onClick={connect} disabled={linking}>
              {linking ? 'Waiting for your wallet…' : 'Connect wallet'}
            </button>
          ) : (
            <button className="btn" onClick={refresh} disabled={busy}>
              {busy ? 'Checking…' : 'Check again'}
            </button>
          )}
        </div>

        {linking && (
          <div className="row">
            <div className="row-label">
              <b>Finish in your browser</b>
              <span>
                Lectus opened a page on your own machine. Approve the signature there —
                it moves no SOL, no tokens, and approves nothing.
              </span>
            </div>
          </div>
        )}

        {status && status.kind !== 'unlinked' && (
          <div className="row">
            <div className="row-label">
              <b>Holdings</b>
              <span>
                {status.kind === 'active'
                  ? `${org(status.balance)} — ${usd(status.usd)}`
                  : `${usd(status.usd)} — below the ${usd(floor?.floor_usd ?? 20)} floor`}
              </span>
            </div>
            <a
              className="btn"
              href={`${EXPLORER}${pubkey}`}
              target="_blank"
              rel="noreferrer noopener"
            >
              View on Solscan
            </a>
          </div>
        )}

        <div className="row">
          <div className="row-label">
            <b>What unlocks it</b>
            <span>
              {floor
                ? `${usd(floor.floor_usd)} of ORGANIC${required ? ` — about ${org(required)} at ${usd(floor.price_usd)} each` : ''}. The dollar amount is fixed; the token count follows the live price.`
                : `$20 of ORGANIC. The dollar amount is fixed; the token count follows the live price.`}
            </span>
          </div>
        </div>
      </div>

      {status?.kind === 'grace' && (
        <div className="banner">
          <span>
            Your wallet is below the floor. Dictation keeps working for{' '}
            {status.days_left} more {status.days_left === 1 ? 'day' : 'days'} — top back
            up to {usd(floor?.floor_usd ?? 20)} to keep it.
          </span>
        </div>
      )}

      {status?.kind === 'locked' && (
        <div className="banner">
          <span>
            Dictation is locked: this wallet holds {usd(status.usd)} of ORGANIC. Top up to{' '}
            {usd(floor?.floor_usd ?? 20)} and hit Check again.
          </span>
        </div>
      )}

      {error && <p className="field-hint" style={{ color: 'var(--danger)' }}>{error}</p>}

      {status && status.kind !== 'unlinked' && (
        <button className="btn btn-ghost" onClick={unlink} disabled={busy}>
          Unlink this wallet
        </button>
      )}
    </div>
  );
}
