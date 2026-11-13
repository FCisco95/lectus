import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Navigation } from '../Navigation';
import type { StepProps } from '../useWizardState';
export function AccessibilityStep({ next, back }: StepProps) {
  const [accessible, setAccessible] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true;
    const poll = () => invoke<boolean>('accessibility_status').then(value => { if (active) setAccessible(value); }).catch(() => { if (active) setAccessible(null); });
    void poll(); const timer = window.setInterval(poll, 1500);
    return () => { active = false; window.clearInterval(timer); };
  }, []);
  const action = async (command: string) => {
    setBusy(true); setError('');
    try { await invoke(command); } catch (error) { setError(`Could not complete that action: ${String(error)}`); }
    finally { setBusy(false); }
  };
  return <><h2 tabIndex={-1}>Allow shortcuts &amp; text insertion</h2>
    <p>macOS Accessibility permission lets Lectus detect your shortcut and paste text into other apps.</p>
    <div className="wizard-status" role="status"><span>Accessibility permission</span><span className={`badge ${accessible ? 'ok' : 'warn'}`}>{accessible ? 'Granted' : accessible === null ? 'Not confirmed' : 'Not granted'}</span></div>
    <div className="help-actions"><button className="btn" type="button" disabled={busy} onClick={() => void action('open_accessibility_settings')}>Open System Settings</button>
      <button className="btn" type="button" disabled={busy} onClick={() => void action('relaunch_app')}>Relaunch Lectus</button></div>
    <p className="wizard-callout">If you just granted permission, relaunch Lectus so the shortcut listener can use it. Setup may open again after relaunch.</p>
    {error && <p role="alert">{error}</p>}<Navigation back={back} next={next} label={accessible ? 'Continue' : 'Set up later'} /></>;
}
