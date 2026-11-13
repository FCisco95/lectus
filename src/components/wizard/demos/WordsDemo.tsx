export function WordsDemo() {
  return <div className="wizard-demo" aria-label="Demo: vocabulary and confirmed spellings">
    <div className="demo-word-list" aria-hidden="true"><span className="selected">Mycel</span><span>Magic Eden</span><span>DexScreener</span></div>
    <div className="demo-message demo-word-match"><span>myself</span><span aria-hidden="true">→</span><strong>Mycel</strong></div>
    <p>Confirm a spelling for the languages you choose.</p></div>;
}
