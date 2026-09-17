import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { Config } from './types';

/** Config load + debounced auto-save, shared by the shell and the Settings modal.
 *
 *  Lifted out of Settings.tsx unchanged in behaviour: there is no Save button,
 *  every edit persists 500 ms later, and the window flushes a pending save on
 *  blur then re-reads on focus — the window is hidden, not unmounted, so a
 *  stale in-memory snapshot would otherwise overwrite a model picked from the
 *  tray.
 */
export function useConfig() {
  const [config, setConfig] = useState<Config | null>(null);
  const [status, setStatus] = useState('');
  const [statusKind, setStatusKind] = useState<'' | 'ok' | 'err'>('');
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const statusTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const configRef = useRef<Config | null>(null);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  useEffect(() => {
    const persist = async (next: Config) => {
      try {
        await invoke('save_config', { newConfig: next });
        setStatus('Saved');
        setStatusKind('ok');
        if (statusTimer.current) clearTimeout(statusTimer.current);
        statusTimer.current = setTimeout(() => setStatus(''), 1500);
      } catch (e) {
        setStatus(`${e}`);
        setStatusKind('err');
      }
    };

    const flush = async () => {
      if (saveTimer.current) {
        clearTimeout(saveTimer.current);
        saveTimer.current = null;
        if (configRef.current) await persist(configRef.current);
      }
    };

    const load = () =>
      invoke<Config>('get_config')
        .then((c) => {
          configRef.current = c;
          setConfig(c);
        })
        .catch((e) => {
          setStatus(`Load error: ${e}`);
          setStatusKind('err');
        });
    load();

    const win = getCurrentWebviewWindow();
    const unFocus = win.listen('tauri://focus', () => {
      flush().then(load);
    });
    const unBlur = win.listen('tauri://blur', () => {
      flush();
    });
    const unModel = listen<string>('model-active', (e) => {
      setConfig((c) => {
        if (!c) return c;
        const next = { ...c, model_name: e.payload };
        configRef.current = next;
        return next;
      });
    });
    return () => {
      unFocus.then((f) => f());
      unBlur.then((f) => f());
      unModel.then((f) => f());
    };
  }, []);

  /** Merge a patch into config and schedule the save. */
  const update = (patch: Partial<Config>) => {
    const current = configRef.current;
    if (!current) return;
    const next = { ...current, ...patch };
    configRef.current = next;
    setConfig(next);
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(async () => {
      saveTimer.current = null;
      try {
        await invoke('save_config', { newConfig: next });
        setStatus('Saved');
        setStatusKind('ok');
        if (statusTimer.current) clearTimeout(statusTimer.current);
        statusTimer.current = setTimeout(() => setStatus(''), 1500);
      } catch (e) {
        setStatus(`${e}`);
        setStatusKind('err');
      }
    }, 500);
  };

  return { config, update, status, statusKind };
}
