import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Config } from '../settings/types';
import { TRY_DICTATION_FLAG } from '../wizard/useWizardState';
import { SPOTLIGHTS, nextSpotlight, spotlightAnchor } from './spotlights';
import type { Spotlight } from './spotlights';

/** Settle time before a spotlight appears, so it never lands mid-click. */
const SHOW_DELAY_MS = 900;
/** How often to look again while the post-setup "Try it now" banner is up.
 *  HomePanel clears that flag without an event, so it has to be polled. */
const RECHECK_MS = 1500;

function tryBannerActive() {
  try { return localStorage.getItem(TRY_DICTATION_FLAG) === '1'; } catch { return false; }
}

interface DiscoveryOptions {
  config: Config | null;
  update: (patch: Partial<Config>) => void;
  /** Settings or What's new is open: nothing new may appear on top. */
  overlayOpen: boolean;
}

/** Schedules in-app spotlights and feeds the "Get started" card.
 *
 *  A spotlight shows only after the first dictation, never over another
 *  overlay or the "Try it now" banner, and at most once per Shell mount
 *  (one app session: the window hides rather than unmounts). Closing it any
 *  way, or following its button, records the id in config.seen_spotlights. */
export function useDiscovery({ config, update, overlayOpen }: DiscoveryOptions) {
  const [dictationsTotal, setDictationsTotal] = useState<number | null>(null);
  const [active, setActive] = useState<Spotlight | null>(null);
  const [recheck, setRecheck] = useState(0);
  const shownThisSession = useRef(false);

  useEffect(() => {
    let alive = true;
    const load = () => {
      invoke<{ dictations_total: number }>('get_history_stats', { tzOffsetMin: new Date().getTimezoneOffset() })
        .then((stats) => { if (alive) setDictationsTotal(stats.dictations_total); })
        .catch(() => {});
    };
    load();
    const unAdded = listen('history-added', load);
    return () => {
      alive = false;
      unAdded.then((f) => f());
    };
  }, []);

  const candidateId = config ? nextSpotlight(config.seen_spotlights)?.id : undefined;

  useEffect(() => {
    if (shownThisSession.current || active || !candidateId) return;
    if (dictationsTotal === null || dictationsTotal < 1 || overlayOpen) return;
    if (tryBannerActive() || document.hidden) {
      // Look again cheaply; only a change re-renders the shell.
      const check = () => { if (!tryBannerActive() && !document.hidden) setRecheck((n) => n + 1); };
      const timer = setInterval(check, RECHECK_MS);
      document.addEventListener('visibilitychange', check);
      return () => {
        clearInterval(timer);
        document.removeEventListener('visibilitychange', check);
      };
    }
    const timer = setTimeout(() => {
      const entry = SPOTLIGHTS.find((s) => s.id === candidateId);
      if (!entry || !spotlightAnchor(entry.target)) return;
      if (tryBannerActive()) { setRecheck((n) => n + 1); return; }
      shownThisSession.current = true;
      setActive(entry);
    }, SHOW_DELAY_MS);
    return () => clearTimeout(timer);
  }, [candidateId, dictationsTotal, overlayOpen, active, recheck]);

  const seen = config?.seen_spotlights;
  /** Close the open spotlight and remember it, whichever way it was closed. */
  const finish = useCallback((id: string) => {
    setActive(null);
    const current = seen ?? [];
    if (!current.includes(id)) update({ seen_spotlights: [...current, id] });
  }, [seen, update]);

  return { dictationsTotal, spotlight: active, finish };
}
