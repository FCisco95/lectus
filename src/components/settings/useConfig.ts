import { useCallback, useEffect, useRef, useState } from 'react';
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
  const saveQueue = useRef<Promise<void>>(Promise.resolve());
  const revision = useRef(0);
  const dirty = useRef(false);

  const persist = useCallback((next: Config) => {
    const savedRevision = revision.current;
    setStatus('Saving…');
    setStatusKind('');
    const queued = saveQueue.current.then(async () => {
      try {
        const saved = await invoke<Config>('save_config', { newConfig: next });
        if (savedRevision !== revision.current) return;
        if (saved) { configRef.current = saved; setConfig(saved); }
        dirty.current = false;
        setStatus('Saved');
        setStatusKind('ok');
        if (statusTimer.current) clearTimeout(statusTimer.current);
        statusTimer.current = setTimeout(() => setStatus(''), 1500);
      } catch (e) {
        setStatus(`Changes could not be saved: ${e}`);
        setStatusKind('err');
        throw e;
      }
    });
    // Keep the queue usable after an error, but let focus refresh see failure.
    saveQueue.current = queued.catch(() => {});
    return queued;
  }, []);

  useEffect(() => {
    const flush = async () => {
      if (saveTimer.current) {
        clearTimeout(saveTimer.current);
        saveTimer.current = null;
      }
      await saveQueue.current;
      if (dirty.current && configRef.current) await persist(configRef.current);
    };

    const load = () => {
      if (dirty.current) return Promise.resolve();
      const startedRevision = revision.current;
      return invoke<Config>('get_config')
        .then((c) => {
          if (dirty.current || revision.current !== startedRevision) return;
          configRef.current = c;
          setConfig(c);
        })
        .catch((e) => {
          setStatus(`Load error: ${e}`);
          setStatusKind('err');
        });
    };
    load();

    const win = getCurrentWebviewWindow();
    const unFocus = win.listen('tauri://focus', () => {
      flush().then(load).catch(() => {});
    });
    const unBlur = win.listen('tauri://blur', () => {
      void flush().catch(() => {});
    });
    const unModel = listen<string>('model-active', (e) => {
      revision.current += 1;
      setConfig((c) => {
        if (!c) return c;
        const next = { ...c, model_name: e.payload };
        configRef.current = next;
        return next;
      });
    });
    return () => {
      void flush().catch(() => {});
      if (statusTimer.current) clearTimeout(statusTimer.current);
      unFocus.then((f) => f());
      unBlur.then((f) => f());
      unModel.then((f) => f());
    };
  }, [persist]);

  /** Merge a patch into config and schedule the save. */
  const update = useCallback((patch: Partial<Config>) => {
    const current = configRef.current;
    if (!current) return;
    const next = { ...current, ...patch };
    revision.current += 1;
    dirty.current = true;
    configRef.current = next;
    setConfig(next);
    if (saveTimer.current) clearTimeout(saveTimer.current);
    if (statusTimer.current) clearTimeout(statusTimer.current);
    setStatus('Unsaved changes');
    setStatusKind('');
    saveTimer.current = setTimeout(() => {
      saveTimer.current = null;
      void persist(configRef.current ?? next).catch(() => {});
    }, 500);
  }, [persist]);

  return { config, update, status, statusKind };
}
