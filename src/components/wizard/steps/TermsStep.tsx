import { LegalSetup } from '../../LegalDocuments';
import { ModelDownloadProgress } from '../../settings/ModelDownloadProgress';
import { Navigation } from '../Navigation';
import type { StepProps } from '../useWizardState';
export function TermsStep({ state, next, back }: StepProps) {
  const model = state.models.find(model => model.name === state.modelName);
  return <><h2 tabIndex={-1}>Privacy &amp; terms</h2><LegalSetup onReady={state.setLegalReady} />
    {state.legalReady && <div className="wizard-download" role="region" aria-label="Speech model download">
      <p>{state.modelBusy ? 'Preparing your speech model. You can continue while it downloads.' : model?.downloaded ? 'Your speech model is downloaded.' : 'You can download a speech model later in Models.'}</p>
      {model && <ModelDownloadProgress name={model.name} label={model.display_name} download={model.download} />}
      {state.modelError && <p role="alert" className="inline-error">{state.modelError} You can finish setup and try again in Models.</p>}
    </div>}<Navigation back={back} next={next} disabled={!state.legalReady} /></>;
}
