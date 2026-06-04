import type { PanelProps } from './types';
import { LANGUAGES } from './types';

export function LanguagePanel({ config, update }: PanelProps) {
  return (
    <div>
      <h2 className="settings-panel-title">Language</h2>
      <p className="settings-panel-sub">
        Lectus auto-detects the spoken language. Override it here if you always dictate in one
        language. To change the Whisper model, go to the <strong>Models</strong> tab.
      </p>

      <div className="field">
        <label className="field-label">Spoken language</label>
        <select
          className="select"
          value={config.language || 'auto'}
          onChange={(e) => update({ language: e.target.value })}
        >
          {LANGUAGES.map((l) => (
            <option key={l.code} value={l.code}>{l.label}</option>
          ))}
        </select>
        <div className="field-hint">Auto-detect handles mixed English/Portuguese seamlessly.</div>
      </div>
    </div>
  );
}
