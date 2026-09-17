import { invoke } from '@tauri-apps/api/core';
import { Icon } from './Icon';
import type { IconName } from './Icon';

/** The surfaces a person looks at. Anything configurable lives in the Settings
 *  modal instead — see SettingsModal.tsx. */
export type SurfaceId = 'home' | 'dictations' | 'vocabulary' | 'membership';

const SURFACES: Array<{ id: SurfaceId; label: string; icon: IconName }> = [
  { id: 'home', label: 'Home', icon: 'home' },
  { id: 'dictations', label: 'Dictations', icon: 'history' },
  { id: 'vocabulary', label: 'Vocabulary', icon: 'vocabulary' },
  { id: 'membership', label: 'Membership', icon: 'key' },
];

interface SidebarProps {
  active: SurfaceId;
  onSelect: (id: SurfaceId) => void;
  onOpenSettings: () => void;
  version: string;
}

export function Sidebar({ active, onSelect, onOpenSettings, version }: SidebarProps) {
  return (
    <nav className="sidebar">
      <div className="sidebar-group">
        {SURFACES.map((s) => (
          <button
            key={s.id}
            type="button"
            className={`sidebar-item${active === s.id ? ' active' : ''}`}
            aria-current={active === s.id ? 'page' : undefined}
            onClick={() => onSelect(s.id)}
          >
            <span className="sidebar-icon"><Icon name={s.icon} /></span>
            {s.label}
          </button>
        ))}
      </div>

      <div className="sidebar-foot">
        <button
          type="button"
          className="sidebar-item"
          onClick={onOpenSettings}
          data-settings-trigger
        >
          <span className="sidebar-icon"><Icon name="general" /></span>
          Settings
        </button>
        <button
          type="button"
          className="sidebar-item"
          // The README is the manual. A fixed URL on the Rust side, so the
          // webview never gets to pick what the system browser opens.
          onClick={() => { invoke('open_help').catch(() => {}); }}
        >
          <span className="sidebar-icon"><Icon name="help" /></span>
          Help
        </button>
        <span className="sidebar-version">v{version}</span>
      </div>
    </nav>
  );
}
