import type { StepProps } from '../useWizardState';
export function WelcomeStep({ state, next }: StepProps) {
  return <><h2 tabIndex={-1}>Talk. Lectus types.</h2>
    <p>Speak into any app. Lectus turns your voice into text on this computer.</p>
    <fieldset className="wizard-language"><legend>I speak</legend>
      {([['en', 'English'], ['pt', 'Portuguese'], ['both', 'Both']] as const).map(([value, label]) =>
        <label key={value} className={state.choice === value ? 'selected' : ''}><input type="radio" name="spoken-language" value={value} checked={state.choice === value} onChange={() => state.setChoice(value)} />{label}</label>)}
    </fieldset>
    <p className="field-hint">The microphone stays off until you ask for a test. Cloud processing is optional and starts off.</p>
    {!state.config && !state.error && <p role="status">Loading your preferences...</p>}
    {state.error && !state.config && <button type="button" className="btn" onClick={state.retry}>Retry settings</button>}
    <div className="wizard-navigation"><button type="button" className="btn btn-primary" disabled={!state.config || state.busy} onClick={() => void state.saveLanguage(next)}>{state.busy ? 'Saving...' : 'Continue'}</button></div></>;
}
