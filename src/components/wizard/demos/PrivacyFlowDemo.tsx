export function PrivacyFlowDemo() {
  return <div className="wizard-demo demo-privacy" aria-label="Demo: local audio processing">
    <div className="demo-flow"><span>Microphone</span><span aria-hidden="true">↓</span><strong>This computer</strong><span aria-hidden="true">↓</span><span>Your text</span></div>
    <p className="demo-cloud">Cloud, only if you turn it on</p><p>Your voice stays on this computer with local speech.</p></div>;
}
