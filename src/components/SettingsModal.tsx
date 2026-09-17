import { useEffect, useRef, useState } from 'react';
import { Icon } from './Icon';
import type { IconName } from './Icon';
import type { Config } from './settings/types';
import { GeneralPanel } from './settings/GeneralPanel';
import { DictationPanel } from './settings/DictationPanel';
import { AIPanel } from './settings/AIPanel';
import { ModelsPanel } from './settings/ModelsPanel';

/** Everything configurable. The four panels are reused unchanged — only their
 *  container moved. */
export type SettingsTab = 'general' | 'dictation' | 'models' | 'ai';

const TABS: Array<{ id: SettingsTab; label: string; icon: IconName }> = [
  { id: 'general', label: 'General', icon: 'general' },
  { id: 'dictation', label: 'Dictation', icon: 'dictation' },
  { id: 'models', label: 'Models', icon: 'models' },
  { id: 'ai', label: 'AI cleanup', icon: 'ai' },
];

interface SettingsModalProps {
  config: Config;
  update: (patch: Partial<Config>) => void;
  initialTab: SettingsTab;
  version: string;
  onClose: () => void;
}

export function SettingsModal({ config, update, initialTab, version, onClose }: SettingsModalProps) {
  const [tab, setTab] = useState<SettingsTab>(initialTab);
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => setTab(initialTab), [initialTab]);

  // Esc closes from anywhere in the window, and focus returns to the gear that
  // opened this so keyboard users are not dropped at the top of the document.
  useEffect(() => {
    panelRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
      const trigger = document.querySelector<HTMLElement>('[data-settings-trigger]');
      trigger?.focus();
    };
  }, [onClose]);

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div
        className="modal-panel"
        role="dialog"
        aria-label="Settings"
        aria-modal="true"
        tabIndex={-1}
        ref={panelRef}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <aside className="modal-rail">
          <div className="modal-rail-title">Settings</div>
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              className={`sidebar-item${tab === t.id ? ' active' : ''}`}
              onClick={() => setTab(t.id)}
            >
              <span className="sidebar-icon"><Icon name={t.icon} /></span>
              {t.label}
            </button>
          ))}
          <div className="modal-rail-foot">Lectus v{version}</div>
        </aside>

        {/* Outside the scrolling body so it stays pinned to the card's corner.
            A plain multiplication sign, not a Segoe glyph: this one also shows
            on macOS. */}
        <button className="modal-close" type="button" aria-label="Close settings" onClick={onClose}>
          &times;
        </button>

        <div className="modal-body">
          {tab === 'general' && <GeneralPanel config={config} update={update} />}
          {tab === 'dictation' && <DictationPanel config={config} update={update} />}
          {tab === 'models' && <ModelsPanel config={config} update={update} />}
          {tab === 'ai' && <AIPanel config={config} update={update} />}
        </div>
      </div>
    </div>
  );
}
