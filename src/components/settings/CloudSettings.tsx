import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { PanelProps } from './types';

/** Shared credentials serve speech and cleanup independently. Checking reads
 * provider metadata only; it never uploads a recording or a transcript. */
export function CloudSettings({ config, update }: PanelProps) {
  const [checking, setChecking] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState(false);
  const revision = useRef(0);
  const checkingRef = useRef(false);
  useEffect(() => {
    revision.current += 1;
    setMessage('');
    return () => { revision.current += 1; };
  }, [config.cloud_base_url, config.cloud_api_key, config.cloud_transcription_model, config.cloud_cleanup_model,
    config.use_cloud, config.ai_cleanup_enabled, config.ai_cleanup_engine, config.output_language,
    config.translation_engine, config.cloud_translation_model, config.cloud_processing_acknowledged]);

  const check = async () => {
    if (checkingRef.current) return;
    checkingRef.current = true;
    const current = revision.current;
    setChecking(true);
    setMessage('Checking provider…');
    setError(false);
    try {
      const ids = await invoke<string[]>('get_cloud_models', {
        baseUrl: config.cloud_base_url, apiKey: config.cloud_api_key,
      });
      if (revision.current !== current) return;
      const selected = [
        ...(config.use_cloud ? [config.cloud_transcription_model] : []),
        ...(config.ai_cleanup_enabled && config.ai_cleanup_engine === 'groq' ? [config.cloud_cleanup_model] : []),
        ...(config.output_language !== 'same' && config.translation_engine === 'cloud' ? [config.cloud_translation_model] : []),
      ];
      const missing = [...new Set(selected.filter((id) => !ids.includes(id.trim())))];
      const hideKey = (value: string) => config.cloud_api_key.trim()
        ? value.split(config.cloud_api_key.trim()).join('[hidden]') : value;
      setError(missing.length > 0);
      setMessage(missing.length
        ? `Provider reachable, but these models are not listed: ${missing.map(hideKey).join(', ')}. Check the model IDs.`
        : selected.length
          ? 'Provider reachable. Selected models are listed; inference access still needs a real dictation test.'
          : 'Provider reachable. No cloud features are enabled.');
    } catch (e) {
      if (revision.current !== current) return;
      setError(true);
      // Native diagnostics contain only safe recovery guidance. Keep a second
      // boundary here so an unexpected IPC error cannot echo the configured key.
      const key = config.cloud_api_key.trim();
      setMessage(key ? String(e).split(key).join('[hidden]') : String(e));
    } finally { checkingRef.current = false; setChecking(false); }
  };

  return <>
    <div className="row">
      <label className="row-label" htmlFor="cloud-base-url"><b>Provider URL</b><span>Shared by API speech, cleanup and translation.</span></label>
      <input id="cloud-base-url" className="input" type="url" spellCheck={false} aria-describedby="cloud-check-scope" value={config.cloud_base_url}
        onChange={(e) => update({ cloud_base_url: e.target.value, cloud_processing_acknowledged: false })} />
    </div>
    <div className="row">
      <label className="checkbox-row">
        <input type="checkbox" checked={config.cloud_processing_acknowledged === true}
          onChange={(e) => update({ cloud_processing_acknowledged: e.target.checked })} />
        <span className="checkbox-text">Allow the features I enable to send audio or text to this provider
          <small>Speech sends recorded audio. Cleanup and translation send transcript text, even with local speech recognition. The provider's retention and account terms apply. Uncheck to stop future requests.</small>
        </span>
      </label>
    </div>
    <div className="row">
      <label className="row-label" htmlFor="cloud-api-key"><b>API key</b><span>Stored on this device. Used only with your provider.</span></label>
      <input id="cloud-api-key" className="input" type="password" autoComplete="off" spellCheck={false} value={config.cloud_api_key}
        onChange={(e) => update({ cloud_api_key: e.target.value })} />
    </div>
    <div className="row">
      <label className="row-label" htmlFor="cloud-speech-model"><b>Speech model</b><span>Choose a Groq model or enter your provider’s model ID.</span></label>
      <input id="cloud-speech-model" className="input" list="speech-models" spellCheck={false} value={config.cloud_transcription_model}
        onChange={(e) => update({ cloud_transcription_model: e.target.value })} />
      <datalist id="speech-models">
        <option value="whisper-large-v3-turbo">Whisper Large v3 Turbo · faster</option>
        <option value="whisper-large-v3">Whisper Large v3 · accuracy focused</option>
      </datalist>
    </div>
    <div className="row">
      <div className="row-label"><b>Connection check</b><span id="cloud-check-scope">Checks credentials and model listing without sending audio or text. Listing a model does not prove inference access.</span></div>
      <button type="button" className="btn btn-secondary" disabled={checking || !config.cloud_api_key.trim()} onClick={check}>
        {checking ? 'Checking…' : 'Check connection'}
      </button>
    </div>
    {message && <p className={`field-hint${error ? ' status-err' : ''}`} role={error ? 'alert' : 'status'}>{message}</p>}
  </>;
}
