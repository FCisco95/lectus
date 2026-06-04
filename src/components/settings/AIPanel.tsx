import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { PanelProps } from './types';

const TONES = ['neutral', 'formal', 'casual', 'concise'];

export function AIPanel({ config, update }: PanelProps) {
  const [claudeAvailable, setClaudeAvailable] = useState<boolean | null>(null);
  const engine = config.ai_cleanup_engine || 'claude';

  useEffect(() => {
    invoke<boolean>('check_claude_cli').then(setClaudeAvailable).catch(() => setClaudeAvailable(false));
  }, []);

  return (
    <div>
      <h2 className="settings-panel-title">AI cleanup</h2>
      <p className="settings-panel-sub">Optionally polish each transcript — punctuation, capitalization, fewer “um”s.</p>

      <label className="checkbox-row">
        <input
          type="checkbox"
          checked={config.ai_cleanup_enabled}
          onChange={(e) => update({ ai_cleanup_enabled: e.target.checked })}
        />
        <span className="checkbox-text">
          Enable AI cleanup
          <small>Runs after transcription. Adds a little latency.</small>
        </span>
      </label>

      {config.ai_cleanup_enabled && (
        <>
          <div className="field">
            <label className="field-label">Engine</label>
            <div className="radio-group">
              <label className={`radio-card${engine === 'claude' ? ' selected' : ''}`}>
                <input
                  type="radio"
                  name="ai_engine"
                  checked={engine === 'claude'}
                  onChange={() => update({ ai_cleanup_engine: 'claude' })}
                />
                <span>
                  <div className="radio-title">
                    Claude Code CLI{' '}
                    {claudeAvailable === true && <span className="badge ok">detected</span>}
                    {claudeAvailable === false && <span className="badge warn">not found</span>}
                  </div>
                  <div className="radio-desc">Uses your Claude subscription — no API tokens. Best quality, ~1–3s.</div>
                </span>
              </label>
              <label className={`radio-card${engine === 'groq' ? ' selected' : ''}`}>
                <input
                  type="radio"
                  name="ai_engine"
                  checked={engine === 'groq'}
                  onChange={() => update({ ai_cleanup_engine: 'groq' })}
                />
                <span>
                  <div className="radio-title">Groq</div>
                  <div className="radio-desc">Sub-second. Uses your cloud API key (General tab).</div>
                </span>
              </label>
            </div>
            {engine === 'claude' && claudeAvailable === false && (
              <div className="field-hint">
                Claude CLI not found on PATH. Install Claude Code, or switch to Groq. If unavailable,
                cleanup is skipped and the raw transcript is used.
              </div>
            )}
          </div>

          <div className="field">
            <label className="field-label">Tone</label>
            <select
              className="select"
              value={config.ai_cleanup_tone || 'neutral'}
              onChange={(e) => update({ ai_cleanup_tone: e.target.value })}
            >
              {TONES.map((t) => (
                <option key={t} value={t}>{t[0].toUpperCase() + t.slice(1)}</option>
              ))}
            </select>
          </div>
        </>
      )}
    </div>
  );
}
