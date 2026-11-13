import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { PanelProps } from './types';
import { IS_MAC, LANGUAGES } from './types';
import { HotkeyCapture } from './HotkeyCapture';
import { HelpTip } from '../HelpTip';
import { ModelDownloadProgress, type DownloadState } from './ModelDownloadProgress';

interface TranslationTarget { code: string; label: string }
interface TranslationModel { name: string; display_name: string; size_mb: number; downloaded: boolean; download?: DownloadState }

export function DictationPanel({ config, update }: PanelProps) {
  const mode = config.trigger_mode || 'hold';
  const [targets, setTargets] = useState<TranslationTarget[]>([]);
  const [targetsError, setTargetsError] = useState('');
  const [targetsReady, setTargetsReady] = useState(false);
  const [model, setModel] = useState<TranslationModel | null>(null);
  const [modelError, setModelError] = useState('');
  const [downloading, setDownloading] = useState(false);
  const [downloadState, setDownloadState] = useState<DownloadState | null>(null);
  const [retry, setRetry] = useState(0);
  const mounted = useRef(false);
  const modelName = useRef('');
  const translating = !!config.output_language && config.output_language !== 'same';

  useEffect(() => {
    mounted.current = true;
    let active = true;
    setTargetsError('');
    setTargetsReady(false);
    setModelError('');
    invoke<TranslationTarget[]>('translation_targets').then((value) => { if (active) setTargets(value); })
      .catch(() => { if (active) setTargetsError('Could not load output languages. Try again.'); })
      .finally(() => { if (active) setTargetsReady(true); });
    const refreshModel = () => invoke<TranslationModel>('get_translation_model_status')
      .then((value) => { if (active) { setModel(value); modelName.current = value.name; if (value.download) setDownloadState(value.download); setModelError(''); } })
      .catch(() => { if (active) setModelError('Could not check the local translation model. Try again.'); });
    refreshModel();
    const unlisten = listen('model-ready', refreshModel);
    const unDownload = listen<{ name: string; status: DownloadState }>('model-download-state', ({ payload }) => {
      if (active && payload.name === modelName.current) {
        setDownloadState(payload.status);
        if (!payload.status.error) setModelError('');
      }
    });
    const timer = window.setInterval(refreshModel, 1500);
    return () => { active = false; mounted.current = false; window.clearInterval(timer); unlisten.then((stop) => stop()); unDownload.then((stop) => stop()); };
  }, [retry]);

  const downloadTranslationModel = async () => {
    setDownloading(true);
    setModelError('');
    try {
      await invoke('download_translation_model');
      const value = await invoke<TranslationModel>('get_translation_model_status');
      if (mounted.current) setModel(value);
    } catch (error) {
      if (mounted.current) {
        try {
          const value = await invoke<TranslationModel>('get_translation_model_status');
          if (mounted.current) {
            setModel(value); setDownloadState(value.download ?? null);
            if (!value.download?.paused && !value.download?.error) setModelError(`Download failed: ${String(error)}`);
          }
        } catch { if (mounted.current) setModelError(`Could not check the saved download: ${String(error)}`); }
      }
    }
    finally { if (mounted.current) setDownloading(false); }
  };
  const downloadBusy = downloading || !!downloadState?.downloading;
  return (
    <div>
      <h2 className="settings-panel-title">Dictation</h2>
      <p className="settings-panel-sub">Choose what you speak, what gets written and how you start recording.</p>

      <div className="field">
        <label className="field-label" htmlFor="spoken-language">Spoken language</label>
        <select
          id="spoken-language"
          className="select"
          value={config.language || 'auto'}
          onChange={(e) => update({ language: e.target.value })}
        >
          {LANGUAGES.map((l) => (
            <option key={l.code} value={l.code}>{l.label}</option>
          ))}
        </select>
        <div className="field-hint">
          Auto-detect lets the model choose the language. Choose a local speech model in
          <strong> Models</strong>, or a cloud speech model in <strong>General</strong>.
        </div>
      </div>

      <HelpTip title="spoken language" why="Help the speech model recognise the language in your recording. This does not translate it."
        setup="Use Auto-detect for mixed use, or choose the language you speak. For Chinese, use Chinese with a multilingual speech model."
        privacy="Your speech setting does not change where audio is processed. General selects local or cloud speech recognition." />

      <div className="field">
        <label className="field-label" htmlFor="output-language">Output language</label>
        <select id="output-language" className="select" value={config.output_language || 'same'}
          onChange={(e) => update({ output_language: e.target.value })}>
          <option value="same">Same as spoken language · no translation</option>
          {translating && !targets.some((target) => target.code === config.output_language) &&
            <option value={config.output_language}>{config.output_language} · current selection</option>}
          {targets.filter((target) => target.code !== 'same').map((target) =>
            <option key={target.code} value={target.code}>{target.label}</option>)}
        </select>
        {!targetsReady && <p className="field-hint" role="status">Loading output languages…</p>}
        {targetsError && <div className="inline-error" role="alert"><span>{targetsError}</span>
          <button type="button" className="btn" onClick={() => setRetry((value) => value + 1)}>Try again</button></div>}
      </div>

      {translating && <>
        <div className="field">
          <label className="field-label" htmlFor="translation-engine">Translation engine</label>
          <select id="translation-engine" className="select" value={config.translation_engine || 'local'}
            onChange={(e) => update({ translation_engine: e.target.value })}>
            <option value="local">Local · text stays on this computer</option>
            <option value="cloud">Cloud API · sends transcript to your provider</option>
            <option value="chatgpt">ChatGPT subscription · text processing</option>
            <option value="claude">Claude subscription · text processing</option>
          </select>
        </div>
        {config.translation_engine === 'cloud' ? <div className="field">
          <label className="field-label" htmlFor="cloud-translation-model">Cloud translation model</label>
          <input id="cloud-translation-model" className="input" list="translation-cloud-models" spellCheck={false}
            value={config.cloud_translation_model ?? 'openai/gpt-oss-120b'} onChange={(e) => update({ cloud_translation_model: e.target.value })} />
          <datalist id="translation-cloud-models"><option value="openai/gpt-oss-120b" /><option value="openai/gpt-oss-20b" /></datalist>
          <p className="field-hint">Cloud translation sends your transcript to the provider configured in General, including when speech recognition is local. Explicit cloud-processing permission in General, a supported chat model and an API key are required.</p>
        </div> : config.translation_engine === 'chatgpt' || config.translation_engine === 'claude' ? <div className="field">
          <p className="field-hint">{config.translation_engine === 'chatgpt' ? 'ChatGPT' : 'Claude'} translates transcript text using the account, shared text model and permission in AI cleanup → Subscription accounts. Connect and test the account there. Choosing translation does not enable cleanup. Provider usage limits apply; there is no paid API fallback.</p>
          <p className="field-hint">Speech recognition stays local or uses your separately billed API provider. Translation failure stops insertion; recover the original transcript from Dictations.</p>
        </div> : <div className="field">
          <div className="field-label">Local translation model</div>
          {model ? <>
            <p className="field-hint">{model.display_name} · {model.size_mb.toLocaleString()} MB · {model.downloaded ? 'Downloaded' : 'Download required'}</p>
            {!model.downloaded && <button className="btn" type="button" disabled={downloadBusy} onClick={downloadTranslationModel}>
              {downloadBusy ? 'Downloading translation model…' : downloadState?.paused || !!downloadState?.downloaded_bytes ? 'Resume translation download' : 'Download translation model'}</button>}
            <ModelDownloadProgress name={model.name} label={model.display_name} download={downloadState ?? undefined} />
          </> : !modelError && <p className="field-hint" role="status">Checking local translation model…</p>}
          {modelError && <div className="inline-error" role="alert"><span>{modelError}</span>
            <button className="btn" type="button" disabled={downloadBusy} onClick={() => setRetry((value) => value + 1)}>Check again</button></div>}
        </div>}
      </>}
      <HelpTip title="output language" why="Translate the recognised transcript before it is inserted. Leave translation off to keep the language you spoke."
        setup="Choose a target language, then local, cloud API or subscription translation. Download the local model, configure the API provider in General, or connect a subscription in AI cleanup. Test a short sentence before relying on a new language pair."
        privacy="Local translation keeps text on this computer. API and subscription translation upload text. If translation fails, recover the original transcript from Dictations." />

      <div className="field">
        <div className="field-label" id="trigger-mode-label">Trigger mode</div>
        <div className="radio-group" role="radiogroup" aria-labelledby="trigger-mode-label">
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

      <div className="field" role="group" aria-labelledby="hold-key-label">
        <div className="field-label" id="hold-key-label">Hold-to-talk key</div>
        <HotkeyCapture value={config.hold_hotkey} onCapture={(k) => update({ hold_hotkey: k })} />
        <div className="field-hint">
          {IS_MAC
            ? 'Used in hold mode. A modifier (⌃ ⇧ ⌥ ⌘), a two-key combo (hold both, e.g. ⌃⌘), or F13–F15.'
            : 'Used in hold mode. A modifier (Ctrl/Shift/Alt/Win), a two-key combo (hold both, e.g. Ctrl+Win), or F13–F15.'}
        </div>
      </div>

      <div className="field" role="group" aria-labelledby="toggle-key-label">
        <div className="field-label" id="toggle-key-label">Toggle key</div>
        <HotkeyCapture value={config.toggle_hotkey} onCapture={(k) => update({ toggle_hotkey: k })} />
        <div className="field-hint">Used in toggle mode. F13–F15 recommended to avoid clashes.</div>
      </div>

      <HelpTip title="recording shortcuts" why="Use hold mode for short phrases or toggle mode when holding a key is uncomfortable."
        setup="Select a mode, then click its key field and press the new key or supported combination. The floating pill opens Home; use the shortcut to record."
        privacy="After setup, with microphone access, Lectus keeps up to half a second of audio in local memory to help retain the first word. Receive mode turns local recording off." />
    </div>
  );
}
