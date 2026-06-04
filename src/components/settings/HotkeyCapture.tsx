import { useState } from 'react';
import { CODE_TO_KEY } from './types';

interface HotkeyCaptureProps {
  value: string;
  onCapture: (key: string) => void;
}

/** A button that captures the next supported key press into a config key string. */
export function HotkeyCapture({ value, onCapture }: HotkeyCaptureProps) {
  const [capturing, setCapturing] = useState(false);
  const [hint, setHint] = useState('');

  return (
    <>
      <button
        type="button"
        className={`btn btn-key${capturing ? ' capturing' : ''}`}
        onClick={() => { setCapturing(true); setHint('Press a key…'); }}
        onKeyDown={(e) => {
          if (!capturing) return;
          e.preventDefault();
          const mapped = CODE_TO_KEY[e.code];
          setCapturing(false);
          if (!mapped) {
            setHint(`Unsupported key (${e.code}). Try Right Ctrl/Shift/Alt or F13–F15.`);
            return;
          }
          onCapture(mapped);
          setHint('');
        }}
      >
        {capturing ? 'Press a key…' : value}
      </button>
      {hint && <div className="field-hint">{hint}</div>}
    </>
  );
}
