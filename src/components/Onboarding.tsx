import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import '../styles/settings.css';
import '../styles/onboarding.css';
import type { Config, LicenseFloor, LicenseStatus } from './settings/types';
import { IS_MAC, allowsDictation, formatHotkey, shortAddress } from './settings/types';
import { HotkeyCapture } from './settings/HotkeyCapture';
import { BrandMark } from './BrandMark';
import { HelpTip } from './HelpTip';
import { LegalSetup, PrivacySummary } from './LegalDocuments';

type StepId = 'welcome' | 'privacy' | 'wallet' | 'mic' | 'accessibility' | 'hotkey' | 'done';
const ALL_STEPS: StepId[] = ['welcome', 'privacy', 'wallet', 'mic', 'accessibility', 'hotkey', 'done'];
const STEPS = IS_MAC ? ALL_STEPS : ALL_STEPS.filter((step) => step !== 'accessibility');
const LABELS: Record<StepId, string> = { welcome: 'Welcome', privacy: 'Privacy & terms', wallet: 'Membership', mic: 'Microphone', accessibility: 'Accessibility', hotkey: 'Shortcut', done: 'Review' };

export function Onboarding({ onComplete }: { onComplete: () => void }) {
  const [config, setConfig] = useState<Config | null>(null);
  const [stepIndex, setStepIndex] = useState(0);
  const [micStatus, setMicStatus] = useState('unknown');
  const [accessible, setAccessible] = useState<boolean | null>(null);
  const [license, setLicense] = useState<LicenseStatus | null>(null);
  const [floor, setFloor] = useState<LicenseFloor | null>(null);
  const [linking, setLinking] = useState(false);
  const [linkError, setLinkError] = useState('');
  const [loadError, setLoadError] = useState('');
  const [actionError, setActionError] = useState('');
  const [actionMessage, setActionMessage] = useState('');
  const [actionBusy, setActionBusy] = useState(false);
  const [finishing, setFinishing] = useState(false);
  const [legalReady, setLegalReady] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const step = STEPS[stepIndex];
  const toggleMode = config?.trigger_mode === 'toggle';
  const selectedHotkey = config ? (toggleMode ? config.toggle_hotkey : config.hold_hotkey) : '';

  useEffect(() => {
    let active = true;
    setLoadError('');
    invoke<Config>('get_config').then((value) => { if (active) setConfig(value); })
      .catch(() => { if (active) setLoadError('Could not load your settings. Retry before continuing.'); });
    invoke<LicenseStatus>('license_status').then((value) => { if (active) setLicense(value); })
      .catch(() => { if (active) setLinkError('Membership status is unavailable. Retry linking or set it up later.'); });
    invoke<LicenseFloor>('license_floor').then((value) => { if (active) setFloor(value); }).catch(() => {});
    return () => { active = false; };
  }, [attempt]);

  useEffect(() => {
    setActionError(''); setActionMessage('');
    if (step === 'privacy') setLegalReady(false);
    headingRef.current?.focus();
    if (rootRef.current) rootRef.current.scrollTop = 0;
  }, [step]);

  useEffect(() => {
    if (step !== 'mic' && !(IS_MAC && step === 'accessibility')) return;
    let active = true;
    const poll = () => {
      if (step === 'mic') invoke<string>('microphone_status').then((value) => { if (active) setMicStatus(value); })
        .catch(() => { if (active) setMicStatus('unknown'); });
      else invoke<boolean>('accessibility_status').then((value) => { if (active) setAccessible(value); })
        .catch(() => { if (active) setAccessible(null); });
    };
    poll();
    const timer = setInterval(poll, 1500);
    return () => { active = false; clearInterval(timer); };
  }, [step]);

  const goNext = () => setStepIndex((value) => Math.min(value + 1, STEPS.length - 1));
  const goBack = () => setStepIndex((value) => Math.max(value - 1, 0));
  const connectWallet = async () => {
    setLinkError(''); setLinking(true);
    try { setLicense(await invoke<LicenseStatus>('link_wallet')); }
    catch (error) { setLinkError(String(error)); }
    finally { setLinking(false); }
  };
  const systemAction = async (command: 'request_microphone_access' | 'open_accessibility_settings' | 'relaunch_app') => {
    setActionBusy(true); setActionError(''); setActionMessage('');
    try {
      await invoke(command);
      if (command === 'request_microphone_access') setActionMessage('Access requested. Check any system prompt, then review the microphone status.');
    } catch (error) { setActionError(`Could not complete that action: ${String(error)}`); }
    finally { setActionBusy(false); }
  };
  const finish = async () => {
    if (!config || !legalReady || finishing) return;
    setFinishing(true); setActionError('');
    try {
      // Apply only the chosen shortcut to a fresh snapshot; preserve concurrent settings.
      const current = await invoke<Config>('get_config');
      const keyField = toggleMode ? 'toggle_hotkey' : 'hold_hotkey';
      if (current[keyField] !== selectedHotkey) {
        await invoke('save_config', { newConfig: { ...current, [keyField]: selectedHotkey } });
      }
      await invoke('complete_onboarding');
      onComplete();
    } catch (error) { setActionError(`Setup was not completed: ${String(error)}. Try again.`); }
    finally { setFinishing(false); }
  };
  const navigation = (label = 'Continue', disabled = false) => <div className="onboarding-btn-row">
    <button className="btn" type="button" disabled={actionBusy || linking || finishing} onClick={goBack}>Back</button>
    <button className="btn btn-primary" type="button" disabled={disabled || actionBusy || linking || finishing} onClick={goNext}>{label}</button>
  </div>;

  return <div className="onboarding-root" ref={rootRef}><div className="onboarding-step">
    <div className="onboarding-progress-label">Step {stepIndex + 1} of {STEPS.length} · {LABELS[step]}</div>
    <div className="onboarding-progress" role="progressbar" aria-label="Setup progress" aria-valuemin={1}
      aria-valuemax={STEPS.length} aria-valuenow={stepIndex + 1} aria-valuetext={LABELS[step]}>
      <span style={{ width: `${((stepIndex + 1) / STEPS.length) * 100}%` }} />
    </div>
    {step === 'welcome' && <>
      <BrandMark className="onboarding-orb" />
      <h2 tabIndex={-1} ref={headingRef}>Make room for your voice</h2>
      <p>Lectus turns speech into text in the app you are using. Start with local dictation; add translation, cloud providers or another computer when you need them.</p>
      <p>Setup covers privacy, membership, microphone access and your shortcut. Dictation and its microphone buffer stay off until setup is complete.</p>
      <HelpTip title="setup" why="Make microphone and data choices explicit before your first dictation."
        setup="Follow these steps, then select a downloaded speech model or configure cloud speech. Membership and OS permissions may need separate attention."
        privacy="The microphone step opens an audio stream only if you request the OS permission prompt. After setup, a short local memory buffer helps retain your first word." />
      {loadError ? <div className="inline-error" role="alert"><span>{loadError}</span><button className="btn" type="button" onClick={() => setAttempt((value) => value + 1)}>Retry settings</button></div>
        : !config && <p role="status">Loading your preferences…</p>}
      <div className="onboarding-btn-row"><button className="btn btn-primary" type="button" disabled={!config} onClick={goNext}>Get started</button></div>
    </>}
    {step === 'privacy' && <>
      <h2 tabIndex={-1} ref={headingRef}>Privacy & terms</h2><p>Review the data choices and publisher documents before continuing.</p>
      <LegalSetup onReady={setLegalReady} />{navigation('Continue', !legalReady)}
    </>}
    {step === 'wallet' && <>
      <h2 tabIndex={-1} ref={headingRef}>Link your membership wallet</h2>
      <p>Dictation requires an eligible ORGANIC or Mycel balance. Linking a Solana wallet requests a signed message; it does not transfer SOL or tokens.</p>
      {floor ? <p>The current requirement is ${floor.floor_usd} of an eligible token. Check Membership after setup for current balances.</p>
        : <p className="field-hint">The current requirement could not be loaded. Check Membership after setup.</p>}
      {allowsDictation(license) && license && license.kind !== 'unlinked' ? <p className="onboarding-ok" role="status">Wallet linked: {shortAddress(license.pubkey)}</p>
        : <button className="btn btn-primary" type="button" onClick={connectWallet} disabled={linking}>{linking ? 'Waiting for your wallet…' : 'Connect wallet'}</button>}
      {linking && <p role="status">Review the signature request in the browser tab Lectus opened.</p>}
      {linkError && <p className="onboarding-err" role="alert">{linkError}</p>}
      <p className="field-hint">You can explore without linking now. Dictation remains unavailable until membership is eligible.</p>
      {navigation(allowsDictation(license) ? 'Continue' : 'Set up later')}
    </>}
    {step === 'mic' && <>
      <h2 tabIndex={-1} ref={headingRef}>Check your microphone</h2>
      <p>After setup, Lectus keeps up to half a second of microphone audio in local memory to help retain the first word. Receive mode turns local recording off.</p>
      <div className="onboarding-status" role="status"><span>{IS_MAC ? 'Microphone permission' : 'Input device'}</span><span className={`badge ${micStatus === 'granted' ? 'ok' : 'warn'}`}>
        {micStatus === 'granted' ? (IS_MAC ? 'Granted' : 'Device available') : micStatus === 'denied' ? 'Denied' : 'Not confirmed'}</span></div>
      <p className="field-hint">Requesting access briefly opens the microphone to trigger the system prompt. It does not save a recording.</p>
      <button className="btn" type="button" disabled={actionBusy} onClick={() => systemAction('request_microphone_access')}>{actionBusy ? 'Requesting access…' : 'Request microphone access'}</button>
      {micStatus !== 'granted' && <p className="onboarding-callout">Check {IS_MAC ? 'System Settings → Privacy & Security → Microphone' : 'Windows Settings → Privacy & security → Microphone and your input device'} if access remains unavailable.</p>}
      <p className="field-hint">Choose a specific microphone later in General. Local speech stays on this computer; cloud speech uploads audio when enabled with your permission.</p>
      {navigation(micStatus === 'granted' ? 'Continue' : 'Set up later')}
    </>}
    {step === 'accessibility' && <>
      <h2 tabIndex={-1} ref={headingRef}>Allow shortcuts & text insertion</h2>
      <p>macOS Accessibility permission lets Lectus detect your shortcut and paste text into other apps.</p>
      <div className="onboarding-status" role="status"><span>Accessibility permission</span><span className={`badge ${accessible ? 'ok' : 'warn'}`}>{accessible ? 'Granted' : accessible === null ? 'Not confirmed' : 'Not granted'}</span></div>
      <div className="help-actions"><button className="btn" type="button" disabled={actionBusy} onClick={() => systemAction('open_accessibility_settings')}>Open System Settings</button>
        <button className="btn" type="button" disabled={actionBusy} onClick={() => systemAction('relaunch_app')}>Relaunch Lectus</button></div>
      <p className="onboarding-callout">If you just granted permission, relaunch Lectus so the shortcut listener can use it. Setup may open again after relaunch.</p>
      {navigation(accessible ? 'Continue' : 'Set up later')}
    </>}
    {step === 'hotkey' && config && <>
      <h2 tabIndex={-1} ref={headingRef}>Choose your dictation shortcut</h2>
      <p>{toggleMode ? 'Tap once to start speaking, then tap again to transcribe.' : 'Hold the key while speaking, then release to transcribe.'} You can change modes later in Dictation settings.</p>
      <div className="onboarding-shortcut" role="group" aria-label={toggleMode ? 'Toggle shortcut' : 'Hold-to-talk shortcut'}><HotkeyCapture value={selectedHotkey}
        onCapture={(key) => setConfig({ ...config, [toggleMode ? 'toggle_hotkey' : 'hold_hotkey']: key })} /></div>
      <p className="field-hint">Your selection is saved when you finish. Pick a key that does not conflict with the apps you use.</p>
      {navigation('Review setup')}
    </>}
    {step === 'done' && <>
      <h2 tabIndex={-1} ref={headingRef}>Review your setup</h2>
      <dl className="onboarding-review">
        <div><dt>Shortcut</dt><dd>{config ? `${toggleMode ? 'Tap' : 'Hold'} ${formatHotkey(selectedHotkey)}` : 'Not loaded'}</dd></div>
        <div><dt>Membership</dt><dd>{allowsDictation(license) ? 'Eligible wallet linked' : 'Link an eligible wallet before dictating'}</dd></div>
        <div><dt>Microphone</dt><dd>{micStatus === 'granted' ? (IS_MAC ? 'Permission granted' : 'Device detected; test in your chosen app') : 'Needs attention before dictation'}</dd></div>
        {IS_MAC && <div><dt>Accessibility</dt><dd>{accessible ? 'Permission granted' : 'Needs attention before dictation'}</dd></div>}
      </dl>
      <p>Open Models after setup to select a downloaded speech model, or configure cloud speech in General. Then test a short sentence in a text field.</p>
      <details className="help-tip"><summary>Review privacy & terms</summary><PrivacySummary /></details>
      <div className="onboarding-btn-row"><button className="btn" type="button" disabled={finishing} onClick={goBack}>Back</button>
        <button className="btn btn-primary" type="button" disabled={finishing || !legalReady || !config} onClick={finish}>{finishing ? 'Saving setup…' : 'Finish setup & open Lectus'}</button></div>
    </>}
    {actionError && <p className="onboarding-err" role="alert">{actionError}</p>}
    <p className="onboarding-feedback" role="status">{actionMessage}</p>
  </div></div>;
}
