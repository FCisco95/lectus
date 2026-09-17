import { useEffect, useRef } from 'react';

/** Mirrors `UpdateInfo` in src-tauri/src/updates.rs. */
export interface WhatsNew {
  version: string;
  notes: string | null;
}

interface WhatsNewModalProps {
  info: WhatsNew;
  onClose: () => void;
}

/** Shown once, on the first launch after an update: the release notes the
 *  updater fetched before the restart. The notes are the GitHub release body
 *  as plain text — no markdown rendering, deliberately; a bullet list reads
 *  fine as-is and nothing in the feed is trusted enough to render as HTML. */
export function WhatsNewModal({ info, onClose }: WhatsNewModalProps) {
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    closeRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  const notes = (info.notes ?? '').trim();

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div
        className="modal-panel compact"
        role="dialog"
        aria-label={`What's new in Lectus ${info.version}`}
        aria-modal="true"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <button className="modal-close" type="button" aria-label="Close" onClick={onClose}>
          &times;
        </button>
        <div className="modal-body">
          <h2 className="settings-panel-title">What's new in v{info.version}</h2>
          <p className="settings-panel-sub">Lectus updated itself while you were away.</p>
          <p className="whats-new-notes">
            {notes || 'No release notes came with this build.'}
          </p>
          <div className="whats-new-actions">
            <button className="btn btn-primary" type="button" ref={closeRef} onClick={onClose}>
              Got it
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
