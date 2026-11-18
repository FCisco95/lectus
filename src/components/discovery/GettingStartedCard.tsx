import { useEffect, useState } from 'react';
import { isEnabled as autostartEnabled } from '@tauri-apps/plugin-autostart';
import type { SurfaceId } from '../Sidebar';
import type { SettingsTab } from '../SettingsModal';
import type { Config } from '../settings/types';
import { IS_MAC } from '../settings/types';

/** Heavier than Icon's check, which thins out at this size. */
function CheckGlyph() {
  return (
    <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true" focusable="false">
      <path d="M2.6 6.3l2.2 2.2 4.6-4.9" fill="none" stroke="currentColor" strokeWidth="1.8"
        strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

interface GettingStartedCardProps {
  config: Config;
  /** From get_history_stats; null until it has loaded. */
  dictationsTotal: number | null;
  /** Launch at login is changed in Settings, so look again when it closes. */
  settingsOpen: boolean;
  onSelect: (surface: SurfaceId) => void;
  onOpenSettings: (tab: SettingsTab) => void;
  onHide: () => void;
}

/** Three first steps in the sidebar, each ticked from real state. Gone once
 *  all are done or the person hides it; CSS hides it on a narrow sidebar. */
export function GettingStartedCard({
  config, dictationsTotal, settingsOpen, onSelect, onOpenSettings, onHide,
}: GettingStartedCardProps) {
  const [autostart, setAutostart] = useState<boolean | null>(null);

  useEffect(() => {
    if (settingsOpen) return;
    let alive = true;
    autostartEnabled()
      .then((on) => { if (alive) setAutostart(on); })
      .catch(() => { if (alive) setAutostart(false); });
    return () => { alive = false; };
  }, [settingsOpen]);

  if (config.getting_started_hidden || dictationsTotal === null || autostart === null) return null;

  const steps = [
    { id: 'dictate', label: 'Dictate once', done: dictationsTotal >= 1, go: () => onSelect('home') },
    { id: 'vocabulary', label: 'Add a word to Vocabulary', done: config.dictionary_words.length > 0, go: () => onSelect('vocabulary') },
    { id: 'autostart', label: IS_MAC ? 'Start Lectus at login' : 'Start Lectus with Windows', done: autostart, go: () => onOpenSettings('general') },
  ];
  const done = steps.filter((s) => s.done).length;
  if (done === steps.length) return null;

  return (
    <section className="getting-started" aria-labelledby="getting-started-title">
      <div className="gs-head">
        <h2 id="getting-started-title" className="gs-title">Get started</h2>
        <button type="button" className="gs-hide" onClick={onHide} aria-label="Hide Get started">Hide</button>
      </div>
      <div className="gs-meter">
        <div className="gs-bar" aria-hidden="true">
          <span style={{ width: `${(done / steps.length) * 100}%` }} />
        </div>
        <span className="gs-count">{done} of {steps.length}</span>
      </div>
      <ul className="gs-list">
        {steps.map((step) => (
          <li key={step.id}>
            <button type="button" className={`gs-item${step.done ? ' done' : ''}`} onClick={step.go} data-step={step.id}>
              <span className="gs-check" aria-hidden="true">{step.done && <CheckGlyph />}</span>
              <span className="gs-label">{step.label}</span>
              {step.done && <span className="sr-only"> (done)</span>}
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
