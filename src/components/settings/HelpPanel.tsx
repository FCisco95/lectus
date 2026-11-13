import type { SettingsTab } from '../SettingsModal';
import { HelpTip } from '../HelpTip';
import { PrivacySummary } from '../LegalDocuments';

interface HelpPanelProps {
  onOpenSettings: (tab: SettingsTab) => void;
  onOpenMembership: () => void;
}

export function HelpPanel({ onOpenSettings, onOpenMembership }: HelpPanelProps) {
  return <div className="help-panel">
    <h2 className="settings-panel-title">Help & setup</h2>
    <p className="settings-panel-sub">Start with dictation, then add the features you need. Each setting explains what it changes and where your data goes.</p>

    <section className="help-section" aria-labelledby="help-first-dictation">
      <h3 id="help-first-dictation">Your first dictation</h3>
      <ol>
        <li>Link an eligible wallet in Membership.</li>
        <li>Choose your microphone in General and a downloaded local model in Models, or configure cloud speech.</li>
        <li>Focus a text field in another app. Use the shortcut shown on Home: hold and release, or tap to start and tap to finish.</li>
      </ol>
      <p>The floating Lectus indicator opens Home when clicked. Your hotkey controls dictation.</p>
      <div className="help-actions">
        <button className="btn btn-primary" type="button" onClick={() => onOpenSettings('dictation')}>Dictation settings</button>
        <button className="btn" type="button" onClick={onOpenMembership}>Membership</button>
      </div>
    </section>

    <section className="help-section" aria-labelledby="help-models">
      <h3 id="help-models">Choose a speech model</h3>
      <p>A speech model turns audio into words. Larger models need more memory and can be slower. Compare them with the language, microphone and computer you actually use.</p>
      <HelpTip title="local models" why="Keep speech recognition on this computer and work without a cloud speech account."
        setup="Open Models, download a multilingual model, then select Use this model. Wait for it to become active before dictating."
        privacy="Model downloads need internet. Local speech recognition does not upload your recording. API or subscription cleanup and translation can still send text when you separately enable them." />
      <button className="btn" type="button" onClick={() => onOpenSettings('models')}>Open models</button>
    </section>

    <section className="help-section" aria-labelledby="help-language">
      <h3 id="help-language">Spoken language, Chinese & translation</h3>
      <p>Spoken language describes the audio. Output language describes the text you want. Keep “Same as spoken language” for ordinary dictation, or choose a translation target.</p>
      <p>For Chinese speech, choose Chinese or Auto-detect and a multilingual speech model. English-only models are not appropriate. Translation quality depends on the selected text model and target language.</p>
      <HelpTip title="translation" why="Speak in one language and produce text in another without changing your speech model."
        setup="In Dictation, choose the output language and a local, cloud API or subscription translation engine. Download the local model, configure the API in General, or connect a subscription in AI cleanup."
        privacy="Local translation runs on this computer. API and subscription translation send the transcript to the selected provider, even when speech recognition is local." />
      <p className="field-hint">Qwen3-ASR is an advanced candidate for a compatible custom speech endpoint; it is not a built-in local Lectus model.</p>
      <button className="btn" type="button" onClick={() => onOpenSettings('dictation')}>Language & translation</button>
    </section>

    <section className="help-section" aria-labelledby="help-cloud">
      <h3 id="help-cloud">Cloud speech & AI cleanup</h3>
      <p>Cloud speech sends audio for transcription. AI cleanup edits an existing transcript. The two features can be enabled independently.</p>
      <p>General holds the provider URL, API key and your explicit cloud-processing permission. “Check connection” verifies model listing; it does not prove a real transcription or translation will succeed. Choose model IDs supported by your provider.</p>
      <p>In AI cleanup, Subscription accounts connects an eligible ChatGPT or Claude account through official login. Subscription models process text only; they do not replace the speech model or include API speech billing. Connect, choose a text model, grant text-processing permission, then select cleanup and translation engines separately. A connected account is not an inference test.</p>
      <p>Claude requires the official Claude Code CLI and uses a separate Lectus account directory. Provider plan and usage limits apply, including extra usage enabled with the provider. Lectus does not switch to a paid API when a subscription fails.</p>
      <div className="help-actions">
        <button className="btn" type="button" onClick={() => onOpenSettings('general')}>Cloud connection</button>
        <button className="btn" type="button" onClick={() => onOpenSettings('ai')}>AI cleanup</button>
      </div>
    </section>

    <section className="help-section" aria-labelledby="help-remote">
      <h3 id="help-remote">Dictate to another computer</h3>
      <p>Both computers need Lectus running on the same local network. On the destination, choose Receive in General and copy its connection code. On the microphone computer, choose Send, paste the code and save.</p>
      <p>Test the connection, then focus the destination text field before speaking. Receive mode turns local recording off. Only finished text is sent between the paired computers.</p>
      <button className="btn" type="button" onClick={() => onOpenSettings('general')}>Set dictation destination</button>
    </section>

    <section className="help-section" aria-labelledby="help-api">
      <h3 id="help-api">Connect another app with the local API</h3>
      <p>The API is off by default. Enable and save it, allow the app’s exact browser origin, then copy the token into that app. A token is required on every data request.</p>
      <p>Callers supply 16 kHz mono WAV audio, up to 8 MiB and 120 seconds. This API uses local inference; it cannot open your microphone, paste text or add transcripts to dictation history.</p>
      <button className="btn" type="button" onClick={() => onOpenSettings('api')}>Local API settings</button>
    </section>

    <section className="help-section" aria-labelledby="help-privacy">
      <h3 id="help-privacy">Privacy & terms</h3>
      <PrivacySummary />
    </section>
  </div>;
}
