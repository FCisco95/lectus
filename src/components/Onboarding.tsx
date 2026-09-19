import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import '../styles/settings.css';
import '../styles/onboarding.css';
import type { Config, LicenseFloor, LicenseStatus } from './settings/types';
import { IS_MAC, allowsDictation, formatHotkey, shortAddress } from './settings/types';
import { HotkeyCapture } from './settings/HotkeyCapture';
import { BrandMark } from './BrandMark';

type StepId = 'welcome' | 'wallet' | 'mic' | 'accessibility' | 'hotkey' | 'done';

const ALL_STEPS: StepId[] = ['welcome', 'wallet', 'mic', 'accessibility', 'hotkey', 'done'];
const STEPS = IS_MAC ? ALL_STEPS : ALL_STEPS.filter((s) => s !== 'accessibility');

interface OnboardingProps {
  onComplete: () => void;
}

export function Onboarding({ onComplete }: OnboardingProps) {
  const [config, setConfig] = useState<Config | null>(null);
  const [stepIndex, setStepIndex] = useState(0);
  const [micStatus, setMicStatus] = useState('unknown');
  const [accessible, setAccessible] = useState(false);
  const [license, setLicense] = useState<LicenseStatus | null>(null);
  const [floor, setFloor] = useState<LicenseFloor | null>(null);
  const [linking, setLinking] = useState(false);
  const [linkError, setLinkError] = useState('');

  useEffect(() => {
    invoke<Config>('get_config').then(setConfig).catch(() => {});
    invoke<LicenseStatus>('license_status').then(setLicense).catch(() => {});
    invoke<LicenseFloor>('license_floor').then(setFloor).catch(() => {});
  }, []);

  const connectWallet = async () => {
    setLinkError('');
    setLinking(true);
    try {
      setLicense(await invoke<LicenseStatus>('link_wallet'));
    } catch (e) {
      setLinkError(`${e}`);
    } finally {
      setLinking(false);
    }
  };

  useEffect(() => {
    const poll = () => invoke<string>('microphone_status').then(setMicStatus).catch(() => {});
    poll();
    const id = setInterval(poll, 1500);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    if (!IS_MAC) return;
    const poll = () => invoke<boolean>('accessibility_status').then(setAccessible).catch(() => {});
    poll();
    const id = setInterval(poll, 1000);
    return () => clearInterval(id);
  }, []);

  const step = STEPS[stepIndex];
  const goNext = () => setStepIndex((i) => Math.min(i + 1, STEPS.length - 1));
  const goBack = () => setStepIndex((i) => Math.max(i - 1, 0));

  const finish = () => {
    invoke('complete_onboarding')
      .catch(() => {})
      .finally(onComplete);
  };

  if (micStatus === 'undetermined' && stepIndex === STEPS.indexOf('mic')) {
    // First paint of the mic step: nudge the OS prompt automatically.
    invoke('request_microphone_access').catch(() => {});
  }

  return (
    <div className="onboarding-root">
      <div className="onboarding-step">
        <div className="onboarding-step-tag">
          Step {stepIndex + 1} · {step === 'mic' ? 'Microphone' : step === 'wallet' ? 'Membership' : step === 'accessibility' ? 'Accessibility (macOS)' : step[0].toUpperCase() + step.slice(1)}
        </div>

        {step === 'welcome' && (
          <>
            <BrandMark className="onboarding-orb" />
            <h2>Welcome to <em>Lectus</em></h2>
            <p>
              Speak anywhere. Hold a key, talk, and your words land in whatever app you're using —
              transcribed locally on your machine.
            </p>
            <p>Setup takes about a minute: microphone, permissions, and your hotkey.</p>
            <div className="onboarding-btn-row">
              <button className="btn btn-primary" style={{ marginLeft: 'auto' }} onClick={goNext}>
                Get started
              </button>
            </div>
          </>
        )}

        {step === 'wallet' && (
          <>
            <h2>Link your wallet</h2>
            <p>
              Lectus is free for people who hold ORGANIC or Mycel — no account, no subscription.
              Connect a Solana wallet once and sign a message. It moves no SOL, no tokens,
              and approves nothing.
            </p>
            <p>
              You need {floor ? `$${floor.floor_usd}` : '$20'} of ORGANIC or Mycel
              {floor?.tokens?.length
                ? ` — ${floor.tokens
                    .filter((t) => t.tokens_required != null)
                    .map((t) => `about ${Math.round(t.tokens_required as number).toLocaleString()} ${t.ticker}`)
                    .join(', or ')} today`
                : ''}
              .
            </p>
            {allowsDictation(license) && license && license.kind !== 'unlinked' ? (
              <p className="onboarding-ok">Linked: {shortAddress(license.pubkey)}</p>
            ) : (
              <div style={{ textAlign: 'center', margin: '18px 0 8px' }}>
                <button className="btn btn-primary" onClick={connectWallet} disabled={linking}>
                  {linking ? 'Waiting for your wallet…' : 'Connect wallet'}
                </button>
              </div>
            )}
            {linking && <p>Approve the signature in the browser tab Lectus just opened.</p>}
            {linkError && <p className="onboarding-err">{linkError}</p>}
            <div className="onboarding-btn-row">
              <button className="btn" onClick={goBack}>Back</button>
              <button className="btn btn-primary" style={{ marginLeft: 'auto' }} onClick={goNext}>
                {allowsDictation(license) ? 'Continue' : 'Skip for now'}
              </button>
            </div>
          </>
        )}

        {step === 'mic' && (
          <>
            <h2>Let Lectus hear you</h2>
            <p>Lectus keeps a short pre-roll buffer so it never clips your first word. Audio is processed on-device.</p>
            <div className="onboarding-status">
              🎙️ Microphone access
              <span className={`badge ${micStatus === 'granted' ? 'ok' : 'warn'}`} style={{ marginLeft: 'auto' }}>
                {micStatus === 'granted' ? 'Granted' : micStatus === 'denied' ? 'Denied' : 'Waiting…'}
              </span>
            </div>
            {micStatus !== 'granted' && (
              <p className="onboarding-callout">
                If the system prompt doesn't appear, enable Lectus under Privacy &amp; Security → Microphone.
              </p>
            )}
            <div className="onboarding-btn-row">
              <button className="btn" onClick={goBack}>Back</button>
              <button className="btn btn-primary" style={{ marginLeft: 'auto' }} onClick={goNext}>
                Continue
              </button>
            </div>
          </>
        )}

        {step === 'accessibility' && (
          <>
            <h2>Allow the hotkey &amp; typing</h2>
            <p>macOS requires Accessibility permission for Lectus to listen for your hotkey and paste text into other apps.</p>
            <div className="onboarding-status">
              ♿ Accessibility
              <span className={`badge ${accessible ? 'ok' : 'warn'}`} style={{ marginLeft: 'auto' }}>
                {accessible ? 'Granted' : 'Waiting…'}
              </span>
            </div>
            <p className="onboarding-callout">
              <b>Important:</b> if you just granted this in System Settings, relaunch Lectus so the
              hotkey listener picks it up. Already granted from a previous run (as detected here)?
              Just continue.
            </p>
            <div className="onboarding-btn-row">
              <button className="btn" onClick={() => invoke('open_accessibility_settings')}>
                Open System Settings
              </button>
              {accessible && (
                <button className="btn" onClick={() => invoke('relaunch_app')}>
                  Relaunch instead
                </button>
              )}
              <button
                className="btn btn-primary"
                style={{ marginLeft: 'auto' }}
                disabled={!accessible}
                onClick={goNext}
              >
                Continue
              </button>
            </div>
          </>
        )}

        {step === 'hotkey' && config && (
          <>
            <h2>Your dictation key</h2>
            <p>Hold to talk — release to transcribe. You can change it any time in Settings.</p>
            <div style={{ textAlign: 'center', margin: '18px 0 26px' }}>
              <span className="onboarding-kbd">{formatHotkey(config.hold_hotkey)}</span>
            </div>
            <HotkeyCapture
              value={config.hold_hotkey}
              onCapture={(k) => setConfig({ ...config, hold_hotkey: k })}
            />
            <div className="onboarding-btn-row">
              <button className="btn" onClick={goBack}>Back</button>
              <button className="btn btn-primary" style={{ marginLeft: 'auto' }} onClick={goNext}>
                Continue
              </button>
            </div>
          </>
        )}

        {step === 'done' && (
          <>
            <BrandMark className="onboarding-orb" />
            <h2>You're all set</h2>
            <p>
              Lectus stays in your {IS_MAC ? 'menu bar' : 'system tray'} at login. Click the tray,
              the shortcut, or the pill to open Home. Hold your hotkey anywhere to dictate.
            </p>
            <p>Tip: add app-specific tone &amp; language profiles under Settings → AI Cleanup.</p>
            <div className="onboarding-btn-row">
              <button className="btn btn-primary" style={{ marginLeft: 'auto' }} onClick={finish}>
                Start dictating
              </button>
            </div>
          </>
        )}

        <div className="onboarding-dots">
          {STEPS.map((s, i) => (
            <i key={s} className={i === stepIndex ? 'on' : ''} />
          ))}
        </div>
      </div>
    </div>
  );
}
