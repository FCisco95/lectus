import type { PanelProps } from './types';
import { HotkeyCapture } from './HotkeyCapture';

export function DictationPanel({ config, update }: PanelProps) {
  const mode = config.trigger_mode || 'hold';
  return (
    <div>
      <h2 className="settings-panel-title">Dictation</h2>
      <p className="settings-panel-sub">How you start and stop talking.</p>

      <div className="field">
        <label className="field-label">Trigger mode</label>
        <div className="radio-group">
          <label className={`radio-card${mode === 'hold' ? ' selected' : ''}`}>
            <input
              type="radio"
              name="trigger_mode"
              checked={mode === 'hold'}
              onChange={() => update({ trigger_mode: 'hold' })}
            />
            <span>
              <div className="radio-title">Hold to talk</div>
              <div className="radio-desc">Hold the key while speaking, release to transcribe.</div>
            </span>
          </label>
          <label className={`radio-card${mode === 'toggle' ? ' selected' : ''}`}>
            <input
              type="radio"
              name="trigger_mode"
              checked={mode === 'toggle'}
              onChange={() => update({ trigger_mode: 'toggle' })}
            />
            <span>
              <div className="radio-title">Tap to toggle</div>
              <div className="radio-desc">Tap once to start, tap again to stop. Hands-free.</div>
            </span>
          </label>
        </div>
      </div>

      <div className="field">
        <label className="field-label">Hold-to-talk key</label>
        <HotkeyCapture value={config.hold_hotkey} onCapture={(k) => update({ hold_hotkey: k })} />
        <div className="field-hint">Used in hold mode. A modifier (Ctrl/Shift/Alt/Win), a two-key combo (hold both, e.g. Ctrl+Win), or F13–F15.</div>
      </div>

      <div className="field">
        <label className="field-label">Toggle key</label>
        <HotkeyCapture value={config.toggle_hotkey} onCapture={(k) => update({ toggle_hotkey: k })} />
        <div className="field-hint">Used in toggle mode. F13–F15 recommended to avoid clashes.</div>
      </div>

      <p className="field-hint">Tip: you can also click the floating pill to start or stop.</p>
    </div>
  );
}
