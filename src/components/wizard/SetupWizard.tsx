import { useEffect, useRef, useState } from 'react';
import { BrandMark } from '../BrandMark';
import { IS_MAC } from '../settings/types';
import { useWizardState } from './useWizardState';
import { Navigation } from './Navigation';
import { WelcomeStep } from './steps/WelcomeStep';
import { TermsStep } from './steps/TermsStep';
import { AccessibilityStep } from './steps/AccessibilityStep';
import { TrialStep } from './steps/TrialStep';
import { DictationDemo } from './demos/DictationDemo';
import { PrivacyFlowDemo } from './demos/PrivacyFlowDemo';
import { TrialDemo } from './demos/TrialDemo';
import '../../styles/settings.css';
import '../../styles/wizard.css';
const steps = IS_MAC ? ['welcome', 'terms', 'mic', 'accessibility', 'words', 'shortcut', 'trial'] : ['welcome', 'terms', 'mic', 'words', 'shortcut', 'trial'];
const labels: Record<string, string> = { welcome: 'Welcome', terms: 'Privacy & terms', mic: 'Microphone', accessibility: 'Accessibility', words: 'Your words', shortcut: 'Shortcut', trial: 'Free trial' };
export function SetupWizard({ onComplete }: { onComplete: () => void }) {
  const state = useWizardState(onComplete);
  const [index, setIndex] = useState(0);
  const root = useRef<HTMLDivElement>(null);
  const step = steps[index];
  const next = () => setIndex(value => Math.min(value + 1, steps.length - 1));
  const back = () => setIndex(value => Math.max(value - 1, 0));
  const props = { state, next, back };
  useEffect(() => { root.current?.querySelector<HTMLHeadingElement>('h2')?.focus(); root.current?.scrollTo(0, 0); }, [step]);
  return <div className="wizard-root" ref={root}>
    <header className="wizard-header"><div className="wizard-brand"><BrandMark /> <strong>Lectus</strong></div><span>{index + 1} of {steps.length}</span></header>
    <div className="wizard-progress" role="progressbar" aria-label="Setup progress" aria-valuemin={1} aria-valuemax={steps.length} aria-valuenow={index + 1} aria-valuetext={labels[step]}><span style={{ width: `${(index + 1) / steps.length * 100}%` }} /></div>
    <div className="wizard-panes"><section className="wizard-task" data-wizard-step={step} aria-label={labels[step]}>
      {step === 'welcome' && <WelcomeStep {...props} />}{step === 'terms' && <TermsStep {...props} />}
      {step === 'accessibility' && <AccessibilityStep {...props} />}{step === 'trial' && <TrialStep {...props} />}
      {['mic', 'words', 'shortcut'].includes(step) && <><h2 tabIndex={-1}>{step === 'mic' ? 'Check your microphone' : step === 'words' ? 'Your words' : 'Choose your dictation shortcut'}</h2><p>You can change these settings any time.</p><Navigation back={back} next={next} /></>}
      {state.error && <p className="inline-error" role="alert">{state.error}</p>}
    </section><aside className="wizard-preview" aria-label="How Lectus works">{step === 'terms' ? <PrivacyFlowDemo /> : step === 'trial' ? <TrialDemo /> : <DictationDemo />}</aside></div>
  </div>;
}
