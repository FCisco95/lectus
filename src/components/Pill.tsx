import '../styles/pill.css';

type PillState = 'idle' | 'recording' | 'transcribing';

interface PillProps {
  state: PillState;
}

export function Pill({ state }: PillProps) {
  if (state === 'idle') return null;

  return (
    <div className="pill-container">
      {state === 'recording' && (
        <>
          <div className="pill-wave">
            <span />
            <span />
            <span />
          </div>
          <span className="pill-label">Listening…</span>
        </>
      )}
      {state === 'transcribing' && (
        <span className="pill-label">Transcribing…</span>
      )}
    </div>
  );
}
