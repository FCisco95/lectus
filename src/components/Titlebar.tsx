import { useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { IS_MAC } from './settings/types';

/** The window's own titlebar.
 *
 *  The settings window runs undecorated (`decorations: false`), so this is the
 *  only way to move, maximise or close it — and the only drag handle the window
 *  has. It renders above every screen, onboarding included: without it, a
 *  first-run user would have a window they cannot move.
 *
 *  macOS keeps its native traffic lights, so the buttons are Windows-only.
 *  The glyphs are Segoe Fluent Icons codepoints — the same ones Windows itself
 *  draws, which is cheaper and more accurate than shipping SVGs.
 */
export function Titlebar() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (IS_MAC) return;
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    const sync = () => { win.isMaximized().then(setMaximized).catch(() => {}); };
    sync();
    win.onResized(sync).then((f) => { unlisten = f; }).catch(() => {});
    return () => { unlisten?.(); };
  }, []);

  const win = () => getCurrentWindow();

  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="titlebar-brand" data-tauri-drag-region>
        <div className="titlebar-orb" />
        <span className="titlebar-name">Lectus</span>
      </div>

      {!IS_MAC && (
        <div className="titlebar-controls">
          <button
            className="titlebar-button"
            type="button"
            aria-label="Minimise"
            onClick={() => win().minimize()}
          >
            &#xE921;
          </button>
          <button
            className="titlebar-button"
            type="button"
            aria-label={maximized ? 'Restore' : 'Maximise'}
            onClick={() => win().toggleMaximize()}
          >
            {maximized ? '' : ''}
          </button>
          <button
            className="titlebar-button danger"
            type="button"
            aria-label="Close"
            // close(), not hide(): the CloseRequested handler in lib.rs already
            // prevents the close and hides the window, so this keeps one path.
            onClick={() => win().close()}
          >
            &#xE8BB;
          </button>
        </div>
      )}
    </div>
  );
}
