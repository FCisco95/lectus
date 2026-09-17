import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Window } from '@tauri-apps/api/window';
import { IS_MAC } from './settings/types';

// Not exported by the API package, so derive it from the method signature.
type ResizeDirection = Parameters<Window['startResizeDragging']>[0];

/** Six invisible 6 px strips along the window edges, forwarding a mousedown
 *  to the OS resize loop.
 *
 *  The window is undecorated on Windows, and whether the native resize
 *  borders survive `decorations: false` is a per-build question nobody has
 *  answered at the keyboard yet — see the "unverified" note in HANDOFF.md.
 *  These strips make the answer moot: if the borders work, the strips sit on
 *  top of them and do the same thing; if they do not, the strips are the
 *  only way to resize. macOS keeps its decorations, so nothing renders there.
 *
 *  Four edges plus the two bottom corners. The top corners are skipped on
 *  purpose: at 6 px they would sit exactly where the titlebar drag region
 *  and the window buttons already live. */
const STRIPS: Array<{ dir: ResizeDirection; className: string }> = [
  { dir: 'North', className: 'resize-n' },
  { dir: 'South', className: 'resize-s' },
  { dir: 'East', className: 'resize-e' },
  { dir: 'West', className: 'resize-w' },
  { dir: 'SouthEast', className: 'resize-se' },
  { dir: 'SouthWest', className: 'resize-sw' },
];

export function ResizeHandles() {
  if (IS_MAC) return null;
  return (
    <>
      {STRIPS.map((s) => (
        <div
          key={s.dir}
          className={`resize-strip ${s.className}`}
          aria-hidden="true"
          onMouseDown={(e) => {
            if (e.button !== 0) return;
            e.preventDefault();
            getCurrentWindow().startResizeDragging(s.dir).catch(() => {});
          }}
        />
      ))}
    </>
  );
}
