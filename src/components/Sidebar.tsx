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
        <span className="sidebar-version">v{version}</span>
      </div>
    </nav>
  );
}
