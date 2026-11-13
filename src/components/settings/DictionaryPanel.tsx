import { useState } from 'react';
import type { PanelProps, ReplacementRule, SoundAlike } from './types';
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

  const alikes = config.sound_alikes || [];
  const setAlike = (i: number, patch: Partial<SoundAlike>) =>
    update({ sound_alikes: alikes.map((a, idx) => (idx === i ? { ...a, ...patch } : a)) });
  const toggleAlikeLanguage = (i: number, code: string, on: boolean) => {
    const current = alikes[i].languages;
    setAlike(i, { languages: on ? [...current.filter((l) => l !== code), code] : current.filter((l) => l !== code) });
  };
  const addAlike = () =>
    update({ sound_alikes: [...alikes, { heard: '', word: '', languages: ['pt'] }] });
  const removeAlike = (i: number) =>
    update({ sound_alikes: alikes.filter((_, idx) => idx !== i) });

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

      <div className="field" style={{ maxWidth: 560 }} role="group" aria-labelledby="sound-alikes-label">
        <div className="field-label" id="sound-alikes-label">Sounds like</div>
        {alikes.map((alike, i) => (
          <div className="rule-row rule-row-alike" key={i}>
            <input
              className="input"
              aria-label={`Sounds like ${i + 1}: when Lectus hears`}
              placeholder="When Lectus hears…"
              value={alike.heard}
              onChange={(e) => setAlike(i, { heard: e.target.value })}
            />
            <input
              className="input"
              aria-label={`Sounds like ${i + 1}: write`}
              placeholder="write…"
              value={alike.word}
              onChange={(e) => setAlike(i, { word: e.target.value })}
            />
            <label className="rule-cs">
              <input
                type="checkbox"
                aria-label={`Sounds like ${i + 1}: Portuguese`}
                checked={alike.languages.includes('pt')}
                onChange={(e) => toggleAlikeLanguage(i, 'pt', e.target.checked)}
              />
              Portuguese
            </label>
            <label className="rule-cs">
              <input
                type="checkbox"
                aria-label={`Sounds like ${i + 1}: English`}
                checked={alike.languages.includes('en')}
                onChange={(e) => toggleAlikeLanguage(i, 'en', e.target.checked)}
              />
              English
            </label>
            <button className="icon-btn" type="button" onClick={() => removeAlike(i)} aria-label={`Remove sounds like ${i + 1}${alike.heard ? ` for ${alike.heard}` : ''}`}>×</button>
          </div>
        ))}
        <button className="btn" type="button" onClick={addAlike} style={{ marginTop: 4 }}>+ Add sound-alike</button>
        <div className="field-hint">Use this when a word is heard as a different real word. It only applies in the languages you tick.</div>
      </div>
    </div>
  );
}
