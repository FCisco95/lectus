import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Config, LicenseFloor, LicenseStatus } from '../settings/types';
import type { DownloadState } from '../settings/ModelDownloadProgress';
import { recommendedModel, type LanguageChoice } from './models';

export const TRY_DICTATION_FLAG = 'lectus.try-dictation';
export interface WizardModel {
  name: string; display_name: string; downloaded: boolean; active: boolean;
  loading: boolean; download: DownloadState;
}
export function useWizardState(onComplete: () => void) {
  const [config, setConfig] = useState<Config | null>(null);
  const [choice, setChoice] = useState<LanguageChoice>('both');
  const [license, setLicense] = useState<LicenseStatus | null>(null);
  const [floor, setFloor] = useState<LicenseFloor | null>(null);
  const [legalReady, setLegalReady] = useState(false);
  const [models, setModels] = useState<WizardModel[]>([]);
  const [modelName, setModelName] = useState('');
  const [modelBusy, setModelBusy] = useState(false);
  const [modelError, setModelError] = useState('');
  const [error, setError] = useState('');
  const [linkError, setLinkError] = useState('');
  const [busy, setBusy] = useState(false);
  const [linking, setLinking] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const alive = useRef(true);
  const downloading = useRef(false);
  const requested = useRef('');
  const refreshModels = useCallback(async () => {
    const current = await invoke<WizardModel[]>('get_models_status');
    if (alive.current) setModels(current);
    return current;
  }, []);
  useEffect(() => {
    alive.current = true; setError('');
    invoke<Config>('get_config').then(value => {
      if (!alive.current) return;
      setConfig(value); setChoice(value.language === 'en' ? 'en' : value.language === 'pt' ? 'pt' : 'both');
    }).catch(() => { if (alive.current) setError('Could not load your settings. Retry before continuing.'); });
    invoke<LicenseStatus>('license_status').then(value => { if (alive.current) setLicense(value); })
      .catch(() => { if (alive.current) setLinkError('Membership status is unavailable. Retry linking or set it up later.'); });
    invoke<LicenseFloor>('license_floor').then(value => { if (alive.current) setFloor(value); }).catch(() => {});
    const refresh = () => { refreshModels().catch(() => { if (alive.current) setModelError('Could not check speech models. You can set one up later in Models.'); }); };
    refresh();
    const unlistens = ['model-download-state', 'model-ready', 'model-active', 'model-load-failed'].map(event => listen(event, refresh));
    const timer = window.setInterval(refresh, 1500);
    return () => { alive.current = false; window.clearInterval(timer); unlistens.forEach(unlisten => { unlisten.then(fn => fn()); }); };
  }, [attempt, refreshModels]);
  const savePatch = async (patch: Partial<Config> | ((current: Config) => Partial<Config>)) => {
    const current = await invoke<Config>('get_config');
    const changes = typeof patch === 'function' ? patch(current) : patch;
    const result = await invoke<Config>('save_config', { newConfig: { ...current, ...changes } });
    if (alive.current) setConfig(previous => ({ ...result,
      hold_hotkey: changes.hold_hotkey ?? previous?.hold_hotkey ?? result.hold_hotkey,
      toggle_hotkey: changes.toggle_hotkey ?? previous?.toggle_hotkey ?? result.toggle_hotkey,
    }));
    return result;
  };
  const downloadAndSelect = useCallback(async (name: string) => {
    if (downloading.current) return;
    downloading.current = true; setModelName(name); setModelBusy(true); setModelError('');
    try {
      const current = await refreshModels();
      if (!current.find(model => model.name === name)?.downloaded) await invoke('download_model', { modelName: name });
      await invoke('select_model', { modelName: name }); await refreshModels();
    } catch (error) { if (alive.current) setModelError(String(error)); }
    finally { downloading.current = false; if (alive.current) setModelBusy(false); }
  }, [refreshModels]);
  useEffect(() => {
    if (!legalReady) return;
    const name = recommendedModel(choice);
    if (requested.current === name) return;
    requested.current = name; void downloadAndSelect(name);
  }, [legalReady, choice, downloadAndSelect]);
  const saveLanguage = async (next: () => void) => {
    if (busy) return;
    setBusy(true); setError('');
    try { await savePatch({ language: choice === 'both' ? 'auto' : choice }); next(); }
    catch (error) { setError(`Could not save your language: ${String(error)}`); }
    finally { setBusy(false); }
  };
  const connectWallet = async () => {
    if (linking) return;
    setLinking(true); setLinkError('');
    try { setLicense(await invoke<LicenseStatus>('link_wallet')); }
    catch (error) { setLinkError(String(error)); }
    finally { setLinking(false); }
  };
  const finish = async () => {
    if (!config || !legalReady || busy || linking) return;
    setBusy(true); setError('');
    try {
      const current = await invoke<Config>('get_config');
      const field = config.trigger_mode === 'toggle' ? 'toggle_hotkey' : 'hold_hotkey';
      if (current[field] !== config[field]) await invoke('save_config', { newConfig: { ...current, [field]: config[field] } });
      await invoke('complete_onboarding'); localStorage.setItem(TRY_DICTATION_FLAG, '1'); onComplete();
    } catch (error) { setError(`Setup was not completed: ${String(error)}. Try again.`); }
    finally { if (alive.current) setBusy(false); }
  };
  return { config, setConfig, choice, setChoice, license, floor, legalReady, setLegalReady,
    models, modelName, modelBusy, modelError, downloadAndSelect, error, setError, busy, linking,
    linkError, savePatch, saveLanguage, connectWallet, finish, retry: () => setAttempt(value => value + 1) };
}
export type WizardState = ReturnType<typeof useWizardState>;
export interface StepProps { state: WizardState; next: () => void; back: () => void }
