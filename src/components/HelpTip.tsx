interface HelpTipProps {
  title: string;
  why: string;
  setup: string;
  privacy?: string;
}

/** Native disclosure keeps explanatory help available to keyboard and screen-reader users. */
export function HelpTip({ title, why, setup, privacy }: HelpTipProps) {
  return <details className="help-tip">
    <summary>About {title}</summary>
    <div className="help-tip-body">
      <p><strong>Why use it.</strong> {why}</p>
      <p><strong>How to set it up.</strong> {setup}</p>
      {privacy && <p><strong>Your data.</strong> {privacy}</p>}
    </div>
  </details>;
}
