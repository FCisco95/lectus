import type { ReactNode } from 'react';
import { Icon } from './Icon';
import type { IconName } from './Icon';

/** The surfaces a person looks at. Anything configurable lives in the Settings
 *  modal instead — see SettingsModal.tsx. */
export type SurfaceId = 'home' | 'dictations' | 'vocabulary' | 'membership' | 'help';

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
  /** The "Get started" card, shown above the footer while it has steps left. */
  getStarted?: ReactNode;
}

// `data-surface` and `data-settings-trigger` are the anchors spotlights point
// at (src/components/discovery/spotlights.ts).
export function Sidebar({ active, onSelect, onOpenSettings, version, getStarted }: SidebarProps) {
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
            data-surface={s.id}
          >
            <span className="sidebar-icon"><Icon name={s.icon} /></span>
            {s.label}
          </button>
        ))}
      </div>

      {getStarted}

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
          className={`sidebar-item${active === 'help' ? ' active' : ''}`}
          aria-current={active === 'help' ? 'page' : undefined}
          onClick={() => onSelect('help')}
          data-surface="help"
        >
          <span className="sidebar-icon"><Icon name="help" /></span>
          Help
        </button>
        <span className="sidebar-version">v{version}</span>
      </div>
    </nav>
  );
}
