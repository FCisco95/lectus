import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import '../styles/pill.css';

type PillState = 'idle' | 'recording' | 'transcribing';

interface PillProps {
  state: PillState;
}

// Eclectus-parrot palette — one hue per bar, left→right across the rainbow.
const BAR_COLORS = [
  '#1db584', // green
  '#17a2a2', // cyan
  '#3b82f6', // blue
  '#e91e8c', // magenta
  '#ff6b5a', // coral
  '#ffa500', // gold
  '#1db584', // green (wrap)
];

const BAR_COUNT = BAR_COLORS.length;

// Floating dictation indicator: idle orb, rainbow audio-level bars while recording, spinner while transcribing.
export function Pill({ state }: PillProps) {
  // Latest mic RMS, written by the event listener, read by the rAF loop.
  // A ref (not state) keeps ~31 emits/sec from triggering React re-renders.
  const levelRef = useRef(0);
  // Smoothed level for buttery decay between emits.
  const smoothedRef = useRef(0);
  const barRefs = useRef<Array<HTMLSpanElement | null>>([]);
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    const unlisten = listen<number>('audio-level', (e) => {
      levelRef.current = e.payload ?? 0;
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  // Persist the pill's position after the user drags it. We listen to the
  // window's own `moved` event (reliable even for OS app-region drags) and
  // save once the drag settles, rather than on every pixel.
  useEffect(() => {
    let settle: ReturnType<typeof setTimeout> | undefined;
    let lastX = 0;
    let lastY = 0;
    const unlisten = getCurrentWindow().onMoved(({ payload }) => {
      lastX = payload.x;
      lastY = payload.y;
      if (settle) clearTimeout(settle);
      settle = setTimeout(() => {
        invoke('save_pill_position', { x: lastX, y: lastY }).catch(() => {});
      }, 400);
    });
    return () => {
      if (settle) clearTimeout(settle);
      unlisten.then((f) => f());
    };
  }, []);

  // Manual drag vs click: app-region drag would swallow clicks, so we detect
  // movement ourselves. A real drag hands off to the OS (startDragging); a
  // clean click (no movement) toggles recording.
  const onMouseDown = (e: React.MouseEvent) => {
    if (e.button !== 0) return;
    const startX = e.screenX;
    const startY = e.screenY;
    let dragging = false;
    const onMove = (me: MouseEvent) => {
      if (!dragging && (Math.abs(me.screenX - startX) > 4 || Math.abs(me.screenY - startY) > 4)) {
        dragging = true;
        cleanup();
        getCurrentWindow().startDragging().catch(() => {});
      }
    };
    const onUp = () => {
      cleanup();
      if (!dragging) {
        invoke('toggle_recording').catch(() => {});
      }
    };
    const cleanup = () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  };

  // Animate the bars while recording. The loop self-cancels when not recording.
  useEffect(() => {
    if (state !== 'recording') {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
      smoothedRef.current = 0;
      return;
    }

    const start = performance.now();
    const tick = (now: number) => {
      // Normalize RMS (~0..0.3 for speech) into 0..1 with gain, then smooth.
      const target = Math.min(1, levelRef.current * 8);
      // Asymmetric smoothing: rise fast, fall slow — feels responsive yet fluid.
      const k = target > smoothedRef.current ? 0.5 : 0.12;
      smoothedRef.current += (target - smoothedRef.current) * k;
      const level = smoothedRef.current;

      const t = (now - start) / 1000;
      for (let i = 0; i < BAR_COUNT; i++) {
        const el = barRefs.current[i];
        if (!el) continue;
        // Per-bar phase gives the row a travelling-wave feel even at steady volume.
        const phase = Math.sin(t * 6 + i * 0.7) * 0.5 + 0.5;
        // Bars near the centre react a touch more, like a real meter.
        const centerBias = 1 - Math.abs(i - (BAR_COUNT - 1) / 2) / BAR_COUNT;
        const h = 4 + level * 26 * (0.45 + 0.55 * phase) * (0.7 + 0.3 * centerBias);
        el.style.height = `${h}px`;
      }
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    };
  }, [state]);

  const expanded = state === 'recording' || state === 'transcribing';

  return (
    <div
      className={`pill-root${expanded ? ' pill-expanded' : ''}`}
      onMouseDown={onMouseDown}
    >
      {state === 'idle' && (
        // Idle: a small parrot-coloured orb. Drag to move, click to start.
        <div className="pill-orb" title="Lectus — drag to move, click to dictate" />
      )}
      {state === 'recording' && (
        <div className="pill-container">
          <div className="pill-wave">
            {BAR_COLORS.map((color, i) => (
              <span
                key={i}
                ref={(el) => { barRefs.current[i] = el; }}
                style={{ background: color }}
              />
            ))}
          </div>
          <span className="pill-label">Listening…</span>
        </div>
      )}
      {state === 'transcribing' && (
        <div className="pill-container">
          <span className="pill-orb pill-orb-spin" />
          <span className="pill-label">Transcribing…</span>
        </div>
      )}
    </div>
  );
}
