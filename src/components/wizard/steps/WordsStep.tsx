import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { StepProps } from '../useWizardState';
import { useWizardMic } from '../useWizardMic';

interface ScanReport { candidates: { term: string; count: number; score: number }[]; files_read: number; truncated: boolean }
function heardWord(text: string): string {
  return text.normalize('NFC').replace(/^[\s\p{P}\p{S}]+|[\s\p{P}\p{S}]+$/gu, '').replace(/\s+/g, ' ');
}

export function WordsStep({ state, next, back }: StepProps) {
  const mic = useWizardMic();
  const alive = useRef(true);
  const held = useRef(false);
  const [report, setReport] = useState<ScanReport | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [added, setAdded] = useState<string[]>([]);
  const [index, setIndex] = useState(0);
  const [hasSaved, setHasSaved] = useState(false);
  const [savedSelection, setSavedSelection] = useState(false);
  const [busy, setBusy] = useState(false);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState('');
  const [heard, setHeard] = useState('');
  const [kind, setKind] = useState<'correct' | 'different' | 'invalid' | null>(null);
  const [languages, setLanguages] = useState<string[]>([]);
  const [ruleSaved, setRuleSaved] = useState(false);
  const word = added[index] || '';
  const canCheck = !state.modelBusy && state.models.some(model => model.active && model.downloaded && !model.loading);

  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  useEffect(() => {
    held.current = false; void mic.cancel();
    setHeard(''); setKind(null); setRuleSaved(false); setChecking(false);
    setLanguages(state.choice === 'en' ? [] : ['pt']);
  }, [word, state.choice, mic.cancel]);
  useEffect(() => {
    const blur = () => { if (held.current) { held.current = false; void mic.cancel(); } };
    window.addEventListener('blur', blur);
    return () => window.removeEventListener('blur', blur);
  }, [mic.cancel]);

  const chooseFolder = async () => {
    if (busy) return;
    setBusy(true); setError('');
    try {
      const folder = await open({ directory: true, multiple: false, title: 'Choose a vocabulary folder' });
      if (typeof folder !== 'string' || !alive.current) return;
      setReport(null); setSelected([]); setAdded([]); setHasSaved(false); setSavedSelection(false);
      const result = await invoke<ScanReport>('scan_folder_for_terms', { path: folder });
      if (alive.current) setReport({ ...result, candidates: result.candidates.slice(0, 30) });
    } catch (error) { if (alive.current) setError(String(error)); }
    finally { if (alive.current) setBusy(false); }
  };
  const addWords = async () => {
    if (busy || !selected.length) return;
    setBusy(true); setError('');
    let newWords: string[] = [];
    try {
      await state.savePatch(current => {
        const words = [...current.dictionary_words];
        const keys = new Set(words.map(word => word.toLowerCase()));
        newWords = selected.filter(word => { const key = word.toLowerCase(); if (keys.has(key)) return false; keys.add(key); words.push(word); return true; });
        return { dictionary_words: words };
      });
      if (alive.current) { setAdded(newWords.slice(0, 5)); setIndex(0); setHasSaved(true); setSavedSelection(true); }
    } catch (error) { if (alive.current) setError(String(error)); }
    finally { if (alive.current) setBusy(false); }
  };
  const press = () => {
    if (held.current || busy || checking || !canCheck || !word) return;
    held.current = true; setHeard(''); setKind(null); setRuleSaved(false); setError('');
    setLanguages(state.choice === 'en' ? [] : ['pt']);
    void mic.start(state.config?.input_device);
  };
  const release = async () => {
    if (!held.current) return;
    held.current = false; setChecking(true);
    try {
      const result = await mic.finish(true);
      if (!alive.current || !result) return;
      const heard = heardWord(result.text);
      setHeard(heard);
      if (!heard || heard.split(/\s+/).length > 4) setKind('invalid');
      else setKind(heard.toLowerCase() === heardWord(word).toLowerCase() ? 'correct' : 'different');
    } finally { if (alive.current) setChecking(false); }
  };
  const saveRule = async () => {
    if (busy || !languages.length || kind !== 'different' || ruleSaved) return;
    setBusy(true); setError('');
    try {
      await state.savePatch(current => ({ sound_alikes: [...(current.sound_alikes || []), { heard, word, languages }] }));
      if (alive.current) setRuleSaved(true);
    } catch (error) { if (alive.current) setError(String(error)); }
    finally { if (alive.current) setBusy(false); }
  };
  return <><h2 tabIndex={-1}>Your words</h2><p>Find the names and terms you use in a folder of Markdown or text files.</p>
    <button type="button" className="btn" onClick={chooseFolder} disabled={busy}>{busy ? 'Working...' : 'Choose a folder'}</button>
    <p className="field-hint">Lectus reads these files on this computer only. It keeps the words you tick and nothing else.</p>
    {report && <div className="wizard-scan-results">
      <p className="field-hint">{report.files_read} files read</p>
      {report.truncated && <p className="field-hint">Large folder: showing the best matches from the first files.</p>}
      {report.candidates.length ? <><fieldset className="wizard-suggestions"><legend>Choose the words to keep</legend>
        {report.candidates.map(candidate => <label className={`wizard-word-chip${selected.includes(candidate.term) ? ' selected' : ''}`} key={candidate.term}>
          <input type="checkbox" checked={selected.includes(candidate.term)} disabled={busy} onChange={event => { setSavedSelection(false); setSelected(words => event.target.checked ? [...words, candidate.term] : words.filter(word => word !== candidate.term)); }} />
          <span>{candidate.term}</span></label>)}
      </fieldset><button type="button" className="btn btn-primary" disabled={!selected.length || busy || savedSelection} onClick={addWords}>{savedSelection ? 'Words added' : `Add ${selected.length} ${selected.length === 1 ? 'word' : 'words'}`}</button></>
        : <p>No unusual words found. You can add words later in Dictionary.</p>}
    </div>}
    {word && <section className="wizard-word-check" aria-label="Say it check"><h3>Say: {word}</h3><p className="field-hint">Optional: check what Lectus hears before your first dictation.</p>
      <button type="button" className="btn" disabled={busy || checking || !canCheck}
        onPointerDown={event => { if (event.button !== 0) return; event.preventDefault(); event.currentTarget.setPointerCapture(event.pointerId); press(); }}
        onPointerUp={() => void release()} onPointerCancel={() => { held.current = false; void mic.cancel(); }}
        onLostPointerCapture={() => { if (held.current) { held.current = false; void mic.cancel(); } }}
        onKeyDown={event => { if (['Space', 'Enter'].includes(event.code)) { event.preventDefault(); if (!event.repeat) press(); } }}
        onKeyUp={event => { if (['Space', 'Enter'].includes(event.code)) { event.preventDefault(); void release(); } }}>
        {checking ? 'Listening back...' : mic.recording ? 'Release to hear it' : 'Hold to say word'}
      </button>
      {!canCheck && <p className="field-hint">A speech model is needed for this check. You can add sound-alikes manually in Dictionary.</p>}
      {kind === 'correct' && <p role="status">Heard correctly.</p>}
      {kind === 'invalid' && <p role="status">Could not hear a single word. Try again.</p>}
      {kind === 'different' && !ruleSaved && <div className="wizard-alike"><p>Heard '{heard}'. Write '{word}' instead when I dictate in:</p>
        {(['pt', 'en'] as const).map(language => <label className="checkbox-row" key={language}><input type="checkbox" checked={languages.includes(language)} onChange={event => setLanguages(values => event.target.checked ? [...values, language] : values.filter(value => value !== language))} /><span>{language === 'pt' ? 'Portuguese' : 'English'}</span></label>)}
        <p className="field-hint">Tick only if this is not a word you use.</p><button type="button" className="btn btn-primary" disabled={!languages.length || busy} onClick={saveRule}>Save</button>
      </div>}
      {ruleSaved && <p role="status">Sound-alike saved.</p>}
      <button type="button" className="btn" disabled={mic.recording || checking} onClick={() => setIndex(value => value + 1)}>{index + 1 < added.length ? 'Next word' : 'Done checking'}</button>
    </section>}
    {hasSaved && !word && <p role="status">Your words are saved.</p>}
    {(error || mic.error) && <p role="alert" className="inline-error">{error || mic.error}</p>}
    <div className="wizard-navigation"><button type="button" className="btn" onClick={back}>Back</button><button type="button" className="btn" onClick={next}>Skip</button>
      {hasSaved && <button type="button" className="btn btn-primary" onClick={next}>Continue</button>}</div>
  </>;
}
