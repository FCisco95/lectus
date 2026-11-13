import { useEffect, useRef, useState } from 'react';
import { HotkeyCapture } from '../../settings/HotkeyCapture';
import { CODE_TO_KEY } from '../../settings/types';
import { Navigation } from '../Navigation';
import { useWizardMic } from '../useWizardMic';
import type { StepProps } from '../useWizardState';

export function ShortcutStep({ state, next, back }: StepProps) {
  const mic = useWizardMic();
  const [text, setText] = useState('');
  const held = useRef(new Set<string>());
  const active = useRef(false);
  const toggle = state.config?.trigger_mode === 'toggle';
  const field = toggle ? 'toggle_hotkey' : 'hold_hotkey';
  const shortcut = state.config?.[field] || '';
  useEffect(() => {
    if (mic.error) { active.current = false; held.current.clear(); }
  }, [mic.error]);
  useEffect(() => {
    const keys = shortcut.split('+');
    const finish = () => { void mic.finish(false).then(result => { if (result) setText(text => `${text}${text ? ' ' : ''}${result.text}`); }); };
    const down = (event: KeyboardEvent) => {
      if (event.repeat || !document.hasFocus() || (event.target instanceof Element && event.target.closest('.btn-key'))) return;
      const key = CODE_TO_KEY[event.code]; if (!key) return;
      held.current.add(key);
      if (!keys.every(key => held.current.has(key))) return;
      event.preventDefault();
      if (active.current) { if (toggle) { active.current = false; finish(); } }
      else { active.current = true; void mic.start(state.config?.input_device); }
    };
    const up = (event: KeyboardEvent) => {
      const key = CODE_TO_KEY[event.code]; held.current.delete(key);
      if (!toggle && active.current && keys.includes(key)) { event.preventDefault(); active.current = false; finish(); }
    };
    const blur = () => { held.current.clear(); active.current = false; void mic.cancel(); };
    window.addEventListener('keydown', down); window.addEventListener('keyup', up); window.addEventListener('blur', blur);
    return () => { window.removeEventListener('keydown', down); window.removeEventListener('keyup', up); window.removeEventListener('blur', blur); blur(); };
  }, [shortcut, toggle, state.config?.input_device, mic.start, mic.finish, mic.cancel]);
  return <><h2 tabIndex={-1}>Choose your dictation shortcut</h2>
    <p>{toggle ? 'Tap once to start speaking, then tap again to transcribe.' : 'Hold the key while speaking, then release to transcribe.'} You can change modes later in Dictation settings.</p>
    {state.config && <HotkeyCapture value={shortcut} onCapture={key => state.setConfig(config => config ? { ...config, [field]: key } : config)} />}
    <label className="wizard-field" htmlFor="wizard-try-it">Try it</label>
    <textarea id="wizard-try-it" aria-label="Try it" className="textarea wizard-try" value={text} onChange={event => setText(event.target.value)} placeholder="Click here, then use your shortcut and speak." />
    {mic.recording && <p role="status">Listening...</p>}{mic.error && <p role="alert" className="inline-error">{mic.error}</p>}
    <p className="field-hint">After setup this shortcut works in every app.</p><Navigation back={back} next={next} />
  </>;
}
