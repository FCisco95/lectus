import type { ComponentType } from 'react';
import type { SurfaceId } from '../Sidebar';
import type { SettingsTab } from '../SettingsModal';
import { CleanupArt, DictationsArt, ModelsArt, VocabularyArt } from './illustrations';

/** Where a spotlight points. A sidebar surface anchors to its own item; a
 *  Settings tab has no item of its own, so it anchors to the Settings button
 *  and its primary button opens the modal at that tab. */
export type SpotlightTarget = { surface: SurfaceId } | { settingsTab: SettingsTab };

export interface Spotlight {
  /** Saved in config.seen_spotlights once shown. Never rename or reuse an id:
   *  a new section gets a new id, which is what makes it show to everyone
   *  who has not seen it yet. */
  id: string;
  target: SpotlightTarget;
  title: string;
  body: string;
  cta: string;
  illustration: ComponentType;
}

/** Priority order. One spotlight per app session: the first unseen entry. */
export const SPOTLIGHTS: readonly Spotlight[] = [
  {
    id: 'vocabulary',
    target: { surface: 'vocabulary' },
    title: 'Teach Lectus your words',
    body: 'Add names, jargon and replacement rules once, and every dictation spells them your way.',
    cta: 'Open Vocabulary',
    illustration: VocabularyArt,
  },
  {
    id: 'dictations',
    target: { surface: 'dictations' },
    title: 'Find past dictations',
    body: 'Filter your recent dictations and copy any of them again, all stored on this computer.',
    cta: 'Open Dictations',
    illustration: DictationsArt,
  },
  {
    id: 'models',
    target: { settingsTab: 'models' },
    title: 'Choose your speech model',
    body: 'Models run on this computer: smaller ones use less memory, larger ones can recognize more accurately.',
    cta: 'Open Models',
    illustration: ModelsArt,
  },
  {
    id: 'ai-cleanup',
    target: { settingsTab: 'ai' },
    title: 'Polish and translate',
    body: 'Fix punctuation, casing and filler words after each dictation, and translate from the Dictation tab.',
    cta: 'Open AI cleanup',
    illustration: CleanupArt,
  },
];

export function nextSpotlight(seen: readonly string[] | undefined): Spotlight | null {
  const done = new Set(seen ?? []);
  return SPOTLIGHTS.find((s) => !done.has(s.id)) ?? null;
}

/** The sidebar element a spotlight points at (Sidebar.tsx sets the hooks). */
export function spotlightAnchor(target: SpotlightTarget): HTMLElement | null {
  const selector = 'surface' in target
    ? `.sidebar [data-surface="${target.surface}"]`
    : '.sidebar [data-settings-trigger]';
  return document.querySelector<HTMLElement>(selector);
}
