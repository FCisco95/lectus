import { BrandMark } from '../../BrandMark';
export function DictationDemo() {
  return <div className="wizard-demo" aria-label="Demo: speech becomes text">
    <div className="demo-voice"><BrandMark /><div className="demo-wave" aria-hidden="true">{[1, 2, 3, 4, 5, 6, 7].map(n => <i key={n} style={{ animationDelay: `${n * 90}ms` }} />)}</div></div>
    <div className="demo-message"><span className="demo-typed">Send the notes after lunch.</span><span className="demo-cursor" aria-hidden="true" /></div>
    <p>Hold your shortcut. Speak. Release.</p></div>;
}
