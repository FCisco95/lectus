import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface WizardTranscript { text: string; language: string | null; audio_ms: number; elapsed_ms: number }
// Serialize stream ownership across steps. A late result must not stop a newer test.
let streamQueue: Promise<void> = Promise.resolve();
let streamOwner: symbol | null = null;
function serialize<T>(action: () => Promise<T>): Promise<T> {
  const work = streamQueue.then(action);
  streamQueue = work.then(() => {}, () => {});
  return work;
}
function release(id: symbol | null): Promise<void> {
  return serialize(async () => {
    if (streamOwner !== id && streamOwner !== null) return;
    try { await invoke('wizard_mic_stop'); }
    finally { if (streamOwner === id) streamOwner = null; }
  });
}

export function useWizardMic() {
  const [level, setLevel] = useState(0);
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState('');
  const alive = useRef(true);
  const id = useRef<symbol | null>(null);
  const phase = useRef<'idle' | 'starting' | 'recording' | 'finishing'>('idle');
  const pendingStart = useRef<Promise<void> | null>(null);
  const cancel = useCallback(async () => {
    const current = id.current; id.current = null; phase.current = 'idle';
    if (alive.current) { setRecording(false); setLevel(0); }
    await release(current).catch(error => { if (alive.current && !id.current) setError(String(error)); });
  }, []);
  const start = useCallback(async (device?: string) => {
    if (phase.current !== 'idle') return;
    const current = Symbol('wizard capture'); id.current = current; phase.current = 'starting'; setError('');
    const work = serialize(async () => {
      streamOwner = current;
      await invoke('wizard_mic_start', { device: device || null });
      if (!alive.current || id.current !== current) {
        await invoke('wizard_mic_stop'); streamOwner = null; return;
      }
      phase.current = 'recording'; setRecording(true);
    });
    pendingStart.current = work;
    try { await work; }
    catch (error) {
      if (alive.current && id.current === current) { id.current = null; phase.current = 'idle'; setRecording(false); setError(String(error)); }
      await release(current).catch(() => {});
    } finally { if (pendingStart.current === work) pendingStart.current = null; }
  }, []);
  const finish = useCallback(async (raw: boolean): Promise<WizardTranscript | null> => {
    const current = id.current;
    if (!current) return null;
    await pendingStart.current?.catch(() => {});
    if (id.current !== current || phase.current !== 'recording') return null;
    phase.current = 'finishing'; setRecording(false); setLevel(0); setError('');
    try {
      const result = await invoke<WizardTranscript>('wizard_transcribe', { raw });
      return alive.current && id.current === current ? result : null;
    } catch (error) { if (alive.current && id.current === current) setError(String(error)); return null; }
    finally {
      await release(current).catch(error => { if (alive.current && id.current === current) setError(String(error)); });
      if (id.current === current) { id.current = null; phase.current = 'idle'; if (alive.current) { setRecording(false); setLevel(0); } }
    }
  }, []);
  useEffect(() => {
    alive.current = true;
    const unLevel = listen<number>('wizard-audio-level', event => {
      if (alive.current && id.current && streamOwner === id.current && phase.current === 'recording') setLevel(Number.isFinite(event.payload) ? Math.max(0, Math.min(1, event.payload)) : 0);
    });
    const close = () => { void cancel(); };
    const unClose = listen('tauri://close-requested', close);
    window.addEventListener('pagehide', close);
    return () => { alive.current = false; void cancel(); window.removeEventListener('pagehide', close); unLevel.then(fn => fn()); unClose.then(fn => fn()); };
  }, [cancel]);
  return { level, recording, error, start, finish, cancel };
}
