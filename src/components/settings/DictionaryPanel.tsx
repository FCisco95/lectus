import { useState } from 'react';
import type { PanelProps, ReplacementRule } from './types';
import { HelpTip } from '../HelpTip';

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
      <HelpTip title="vocabulary & replacement rules"
        why="Keep names, specialist words and repeated phrases consistent before cleanup or translation."
        setup="Add the spelling you want, or create a spoken-to-written replacement and enable match case only when needed."
        privacy="Vocabulary is stored locally, but can appear in cloud speech prompts or transcripts when you enable an external processing route." />

      <div className="field">
        <label className="field-label" htmlFor="dictionary-new-word">Custom words</label>
        <div className="tag-input-row">
          <input
            id="dictionary-new-word"
            className="input"
            value={newWord}
            placeholder="e.g. Lectus, Meteora, Tauri…"
            onChange={(e) => setNewWord(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); addWord(); } }}
          />
          <button className="btn" type="button" onClick={addWord} aria-label="Add custom word">Add</button>
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

      <div className="field" style={{ maxWidth: 560 }} role="group" aria-labelledby="replacement-rules-label">
        <div className="field-label" id="replacement-rules-label">Replacement rules</div>
        {rules.map((rule, i) => (
          <div className="rule-row" key={i}>
            <input
              className="input"
              aria-label={`Rule ${i + 1}: spoken text`}
              placeholder="spoken…"
              value={rule.from}
              onChange={(e) => setRule(i, { from: e.target.value })}
            />
            <input
              className="input"
              aria-label={`Rule ${i + 1}: replacement text`}
              placeholder="written…"
              value={rule.to}
              onChange={(e) => setRule(i, { to: e.target.value })}
            />
            <label className="rule-cs">
              <input
                type="checkbox"
                aria-label={`Rule ${i + 1}: match case`}
                checked={rule.case_sensitive}
                onChange={(e) => setRule(i, { case_sensitive: e.target.checked })}
              />
              Aa
            </label>
            <button className="icon-btn" type="button" onClick={() => removeRule(i)} aria-label={`Remove replacement rule ${i + 1}${rule.from ? ` for ${rule.from}` : ''}`}>×</button>
          </div>
        ))}
        <button className="btn" type="button" onClick={addRule} style={{ marginTop: 4 }}>+ Add rule</button>
        <div className="field-hint">e.g. “lectus” → “Lectus”, “at gmail” → “@gmail”. Applied in order.</div>
      </div>
    </div>
  );
}
