import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { IS_MAC } from '../../settings/types';
import { ModelDownloadProgress } from '../../settings/ModelDownloadProgress';
import { Navigation } from '../Navigation';
import { useWizardMic, type WizardTranscript } from '../useWizardMic';
import type { StepProps } from '../useWizardState';
import { ACCURATE_MODEL, isFast } from '../models';

export function MicStep({ state, next, back }: StepProps) {
  const mic = useWizardMic();
  const [devices, setDevices] = useState<string[]>([]);
  const [status, setStatus] = useState('unknown');
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState('');
  const [message, setMessage] = useState('');
  const [heard, setHeard] = useState<WizardTranscript | null>(null);
  const [testModel, setTestModel] = useState('');
  useEffect(() => {
    let active = true;
    invoke<string[]>('list_input_devices').then(devices => { if (active) setDevices(devices); }).catch(error => { if (active) setActionError(String(error)); });
    const poll = () => invoke<string>('microphone_status').then(status => { if (active) setStatus(status); }).catch(() => { if (active) setStatus('unknown'); });
    void poll(); const timer = window.setInterval(poll, 1500);
    return () => { active = false; window.clearInterval(timer); };
  }, []);
  const model = state.models.find(model => model.name === state.modelName);
  const ready = !!model?.downloaded && !model.loading && !state.modelBusy;
  const chooseDevice = async (device: string) => {
    setBusy(true); setActionError('');
    try { await mic.cancel(); await state.savePatch({ input_device: device }); }
    catch (error) { setActionError(String(error)); } finally { setBusy(false); }
  };
  const requestAccess = async () => {
    setBusy(true); setActionError(''); setMessage('');
    try { await invoke('request_microphone_access'); setMessage('Access requested. Check any system prompt, then review the microphone status.'); }
    catch (error) { setActionError(String(error)); } finally { setBusy(false); }
  };
  const test = async () => { setBusy(true); setHeard(null); await mic.start(state.config?.input_device); setBusy(false); };
  const transcribe = async () => {
    setBusy(true); setTestModel(state.models.find(model => model.active)?.name || state.modelName);
    try { const result = await mic.finish(false); if (result) setHeard(result); } finally { setBusy(false); }
  };
  return <><h2 tabIndex={-1}>Check your microphone</h2><p>Say a short sentence, then show the result.</p>
    <label className="wizard-field">Input device<select className="input" aria-label="Input device" value={state.config?.input_device || ''} disabled={busy} onChange={event => void chooseDevice(event.target.value)}>
      <option value="">System default</option>{devices.map(device => <option key={device} value={device}>{device}</option>)}
    </select></label>
    <div className="wizard-status"><span className={`badge ${status === 'granted' ? 'ok' : 'warn'}`} role="status">{status === 'granted' ? (IS_MAC ? 'Granted' : 'Device available') : status === 'denied' ? 'Denied' : 'Not confirmed'}</span>
      <button type="button" className="btn" disabled={busy || mic.recording} onClick={requestAccess}>Request microphone access</button></div>
    {status !== 'granted' && <p className="field-hint">Check {IS_MAC ? 'System Settings, Privacy & Security, Microphone' : 'Windows Settings, Privacy & security, Microphone and your input device'} if access remains unavailable.</p>}
    <div className="wizard-mic-test">
      <div className="wizard-meter" role="progressbar" aria-label="Microphone level" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(mic.level * 100)}><span style={{ width: `${mic.level * 100}%` }} /></div>
      {mic.recording ? <button type="button" className="btn btn-primary" disabled={!ready || busy} onClick={transcribe}>Show what I said</button>
        : <button type="button" className="btn btn-primary" disabled={busy} onClick={test}>{busy ? 'Working...' : 'Test my microphone'}</button>}
    </div>
    {state.modelBusy && <><p role="status">Preparing the speech model. The level bar works while it downloads.</p>{model && <ModelDownloadProgress name={model.name} label={model.display_name} download={model.download} />}</>}
    {!state.modelBusy && !model?.downloaded && <p className="field-hint">The speech model is not downloaded yet. You can test later in Home.</p>}
    {heard && <div className="wizard-callout"><strong>Heard:</strong><p>{heard.text}</p></div>}
    {heard && isFast(testModel, heard.audio_ms, heard.elapsed_ms) && !state.models.some(model => model.name === ACCURATE_MODEL && model.active) && <div className="wizard-callout"><p>This computer is fast. Use the most accurate model (574 MB)?</p>
      <button type="button" className="btn" disabled={state.modelBusy} onClick={() => void state.downloadAndSelect(ACCURATE_MODEL)}>Use the most accurate model</button></div>}
    {(mic.error || actionError) && <p role="alert" className="inline-error">{mic.error || actionError}</p>}
    {message && <p role="status" className="field-hint">{message}</p>}
    <Navigation back={back} next={next} label={mic.error || actionError || status === 'denied' ? 'Set up later' : 'Continue'} />
  </>;
}
