import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
import { spotlightAnchor } from './spotlights';
import type { Spotlight } from './spotlights';

interface SpotlightPopoverProps {
  spotlight: Spotlight;
  /** X, Escape or a click on the dimmed window. */
  onDismiss: () => void;
  /** The primary button, or a click on the highlighted sidebar item. */
  onAccept: () => void;
}

interface Placement {
  top: number;
  left: number;
  /** Arrow centre, measured from the popover's top edge. */
  arrow: number;
  halo: { top: number; left: number; width: number; height: number };
}

const GAP = 14;        // between the item and the popover
const MARGIN = 12;     // minimum distance from the window edge
const HALO_PAD = 3;    // the lit area around the item
const ARROW_INSET = 22; // keep the arrow off the rounded corners

/** One-time popover beside a sidebar item. Rendered outside .shell-columns,
 *  which Shell makes inert while this is open. The dim is the halo's own
 *  shadow, so the item it points at stays lit and clickable. */
export function SpotlightPopover({ spotlight, onDismiss, onAccept }: SpotlightPopoverProps) {
  const popRef = useRef<HTMLDivElement>(null);
  const primaryRef = useRef<HTMLButtonElement>(null);
  const [place, setPlace] = useState<Placement | null>(null);
  const titleId = useId();
  const bodyId = useId();
  // Focus goes back to the item unless the button opened Settings, which
  // takes focus itself.
  const returnFocus = useRef(true);
  const dismissRef = useRef(onDismiss);
  dismissRef.current = onDismiss;
  const targetRef = useRef(spotlight.target);
  targetRef.current = spotlight.target;

  const measure = useCallback(() => {
    const anchor = spotlightAnchor(spotlight.target);
    const pop = popRef.current;
    if (!anchor || !pop) return;
    const r = anchor.getBoundingClientRect();
    const w = pop.offsetWidth;
    const h = pop.offsetHeight;
    // Stay inside the app area, clear of our own titlebar.
    const app = pop.closest('.settings-app')?.getBoundingClientRect();
    const minTop = (app?.top ?? 0) + MARGIN;
    const maxBottom = (app?.bottom ?? window.innerHeight) - MARGIN;
    const left = Math.max(MARGIN, Math.min(r.right + GAP, window.innerWidth - MARGIN - w));
    const centre = r.top + r.height / 2;
    const top = Math.max(minTop, Math.min(centre - h / 2, maxBottom - h));
    const arrow = Math.max(ARROW_INSET, Math.min(centre - top, h - ARROW_INSET));
    setPlace({
      top, left, arrow,
      halo: { top: r.top - HALO_PAD, left: r.left - HALO_PAD, width: r.width + HALO_PAD * 2, height: r.height + HALO_PAD * 2 },
    });
  }, [spotlight.target]);

  useLayoutEffect(() => {
    measure();
    window.addEventListener('resize', measure);
    return () => window.removeEventListener('resize', measure);
  }, [measure]);

  // Hidden until measured, and a hidden button cannot take focus.
  const placed = place !== null;
  useEffect(() => {
    if (placed) primaryRef.current?.focus();
  }, [placed]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        e.preventDefault();
        returnFocus.current = true;
        dismissRef.current();
        return;
      }
      if (e.key !== 'Tab' || !popRef.current) return;
      const nodes = popRef.current.querySelectorAll<HTMLElement>('button:not([disabled])');
      if (nodes.length === 0) return;
      const first = nodes[0];
      const last = nodes[nodes.length - 1];
      const current = document.activeElement;
      const inside = !!current && popRef.current.contains(current);
      if (e.shiftKey && (current === first || !inside)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && (current === last || !inside)) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
      if (!returnFocus.current) return;
      // Shell lifts `inert` in the same commit; focus once that has run.
      const target = targetRef.current;
      requestAnimationFrame(() => spotlightAnchor(target)?.focus());
    };
  }, []);

  const dismiss = () => { returnFocus.current = true; onDismiss(); };
  const accept = () => { returnFocus.current = !('settingsTab' in spotlight.target); onAccept(); };
  const Art = spotlight.illustration;
  const style: CSSProperties = place
    ? { top: place.top, left: place.left, ['--spotlight-arrow' as string]: `${place.arrow}px` }
    : { visibility: 'hidden' };

  return (
    <div className="spotlight-layer">
      <div className="spotlight-backdrop" onMouseDown={dismiss} />
      {place && (
        <div className="spotlight-halo" style={place.halo} onClick={accept} aria-hidden="true" />
      )}
      <div
        ref={popRef}
        className="spotlight"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={bodyId}
        data-spotlight={spotlight.id}
        style={style}
      >
        <span className="spotlight-arrow" aria-hidden="true" />
        <div className="spotlight-art"><Art /></div>
        <div className="spotlight-content">
          <h2 id={titleId} className="spotlight-title">{spotlight.title}</h2>
          <p id={bodyId} className="spotlight-body">{spotlight.body}</p>
          <div className="spotlight-actions">
            <button ref={primaryRef} className="btn btn-primary" type="button" onClick={accept}>
              {spotlight.cta}
            </button>
          </div>
        </div>
        {/* Last in the DOM so Tab runs primary, then close. */}
        <button className="modal-close spotlight-close" type="button" aria-label="Close tip" onClick={dismiss}>
          &times;
        </button>
      </div>
    </div>
  );
}
