import { useState } from 'react';
import { CODE_TO_KEY } from './types';

interface HotkeyCaptureProps {
  value: string;
  onCapture: (key: string) => void;
}

/**
 * A button that captures the next key press — or a two-key chord — into a
 * config key string ("LControl" or "LControl+LWin"). Keys accumulate while
 * held; the first key-up finalizes the combo.
 */
export function HotkeyCapture({ value, onCapture }: HotkeyCaptureProps) {
  const [capturing, setCapturing] = useState(false);
  const [held, setHeld] = useState<string[]>([]);
  const [hint, setHint] = useState('');

  const reset = () => { setCapturing(false); setHeld([]); };

  return (
    <>
      <button
        type="button"
        className={`btn btn-key${capturing ? ' capturing' : ''}`}
        onClick={() => { setCapturing(true); setHeld([]); setHint('Press a key or combo…'); }}
        onBlur={reset}
        onKeyDown={(e) => {
          if (!capturing) return;
          e.preventDefault();
          const mapped = CODE_TO_KEY[e.code];
          if (!mapped) {
            reset();
            setHint(`Unsupported key (${e.code}). Try Ctrl/Shift/Alt/Win or F13–F15.`);
            return;
          }
          if (!held.includes(mapped) && held.length < 2) {
            setHeld([...held, mapped]);
          }
        }}
        onKeyUp={(e) => {
          if (!capturing || held.length === 0) return;
          e.preventDefault();
          onCapture(held.join('+'));
          reset();
          setHint('');
        }}
      >
        {capturing ? (held.length ? `${held.join('+')}…` : 'Press a key or combo…') : value}
      </button>
      {hint && <div className="field-hint">{hint}</div>}
    </>
  );
}
