export function TrialDemo() {
  return <div className="wizard-demo demo-trial" aria-label="Demo: seven day free trial">
    <strong className="demo-seven">7</strong><span>days to make it yours</span>
    <div className="demo-days" aria-hidden="true">{[1, 2, 3, 4, 5, 6, 7].map(day => <span key={day}>{day}</span>)}</div>
    <p>Start today. Connect a wallet when you are ready.</p></div>;
}
