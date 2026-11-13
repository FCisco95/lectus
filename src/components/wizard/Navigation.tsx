export function Navigation({ back, next, disabled = false, label = 'Continue' }: {
  back: () => void; next: () => void; disabled?: boolean; label?: string;
}) {
  return <div className="wizard-navigation"><button type="button" className="btn" onClick={back}>Back</button>
    <button type="button" className="btn btn-primary" onClick={next} disabled={disabled}>{label}</button></div>;
}
