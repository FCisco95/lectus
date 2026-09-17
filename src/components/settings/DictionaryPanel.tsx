import { useState } from 'react';
import type { PanelProps, ReplacementRule } from './types';

export function DictionaryPanel({ config, update }: PanelProps) {
  const [newWord, setNewWord] = useState('');
  const words = config.dictionary_words || [];
  const rules = config.replacement_rules || [];

  const addWord = () => {
    const w = newWord.trim();
    if (!w || words.includes(w)) { setNewWord(''); return; }
    update({ dictionary_words: [...words, w] });
    setNewWord('');
  };
  const removeWord = (w: string) =>
    update({ dictionary_words: words.filter((x) => x !== w) });

  const setRule = (i: number, patch: Partial<ReplacementRule>) => {
    const next = rules.map((r, idx) => (idx === i ? { ...r, ...patch } : r));
    update({ replacement_rules: next });
  };
  const addRule = () =>
    update({ replacement_rules: [...rules, { from: '', to: '', case_sensitive: false }] });
  const removeRule = (i: number) =>
    update({ replacement_rules: rules.filter((_, idx) => idx !== i) });

  return (
    <div>
      <h2 className="settings-panel-title">Dictionary</h2>
      <p className="settings-panel-sub">
        Saved on this machine. After each dictation, these spellings replace whatever Whisper guessed.
      </p>

      <div className="field">
        <label className="field-label">Custom words</label>
        <div className="tag-input-row">
          <input
            className="input"
            value={newWord}
            placeholder="e.g. Lectus, Meteora, Tauri…"
            onChange={(e) => setNewWord(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); addWord(); } }}
          />
          <button className="btn" type="button" onClick={addWord}>Add</button>
        </div>
        <div className="field-hint">
          Stored in Settings and applied to every transcript (e.g. “claude” → “Claude”).
        </div>
        {words.length > 0 && (
          <div className="tags">
            {words.map((w) => (
              <span className="tag" key={w}>
                {w}
                <button type="button" onClick={() => removeWord(w)} aria-label={`Remove ${w}`}>×</button>
              </span>
            ))}
          </div>
        )}
      </div>

      <div className="field" style={{ maxWidth: 560 }}>
        <label className="field-label">Replacement rules</label>
        {rules.map((rule, i) => (
          <div className="rule-row" key={i}>
            <input
              className="input"
              placeholder="spoken…"
              value={rule.from}
              onChange={(e) => setRule(i, { from: e.target.value })}
            />
            <input
              className="input"
              placeholder="written…"
              value={rule.to}
              onChange={(e) => setRule(i, { to: e.target.value })}
            />
            <label className="rule-cs">
              <input
                type="checkbox"
                checked={rule.case_sensitive}
                onChange={(e) => setRule(i, { case_sensitive: e.target.checked })}
              />
              Aa
            </label>
            <button className="icon-btn" type="button" onClick={() => removeRule(i)} aria-label="Remove rule">×</button>
          </div>
        ))}
        <button className="btn" type="button" onClick={addRule} style={{ marginTop: 4 }}>+ Add rule</button>
        <div className="field-hint">e.g. “lectus” → “Lectus”, “at gmail” → “@gmail”. Applied in order.</div>
      </div>
    </div>
  );
}
