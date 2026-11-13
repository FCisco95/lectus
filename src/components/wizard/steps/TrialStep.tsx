import { linkedPubkey, shortAddress } from '../../settings/types';
import type { StepProps } from '../useWizardState';
export function TrialStep({ state, back }: StepProps) {
  const pubkey = linkedPubkey(state.license);
  const eligible = state.license?.kind === 'active';
  return <><h2 tabIndex={-1}>7 days free, starting now</h2>
    <p>After 7 days, dictation needs a wallet holding ${state.floor?.floor_usd ?? 20} of ORGANIC or Mycel. Linking asks for a signed message. It never moves SOL or tokens.</p>
    {!state.floor && <p className="field-hint">Check Membership after setup for the current requirement.</p>}
    {pubkey && <p role="status">Wallet linked: {shortAddress(pubkey)}</p>}
    {!eligible && <button className="btn" type="button" onClick={state.connectWallet} disabled={state.linking || state.busy}>{state.linking ? 'Waiting for your wallet...' : 'Connect wallet'}</button>}
    {state.linking && <p role="status">Review the signature request in the browser tab Lectus opened.</p>}
    {state.linkError && <p role="alert" className="inline-error">{state.linkError}</p>}
    <p className="field-hint">No wallet needed today. You can link one later in Membership.</p>
    <div className="wizard-navigation"><button className="btn" type="button" onClick={back} disabled={state.busy || state.linking}>Back</button>
      <button className="btn btn-primary" type="button" onClick={state.finish} disabled={!state.config || !state.legalReady || state.busy || state.linking}>{state.busy ? 'Saving setup...' : eligible ? 'Finish setup' : 'Start my free trial'}</button></div></>;
}
