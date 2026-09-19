// Mirrors `Config` / `ReplacementRule` in src-tauri/src/config.rs.

export interface ReplacementRule {
  from: string;
  to: string;
  case_sensitive: boolean;
}

// Mirrors `AppProfile` in src-tauri/src/config.rs (null ⇔ Option::None).
export interface AppProfile {
  app_match: string;
  ai_cleanup_enabled: boolean | null;
  ai_cleanup_tone: string | null;
  language: string | null;
  injection_mode: string | null;
}

export interface Config {
  model_path: string;
  use_cloud: boolean;
  cloud_base_url: string;
  cloud_api_key: string;
  hold_hotkey: string;
  toggle_hotkey: string;
  pill_x: number;
  pill_y: number;
  language: string;
  model_name: string;
  trigger_mode: string;
  dictionary_words: string[];
  replacement_rules: ReplacementRule[];
  ai_cleanup_enabled: boolean;
  ai_cleanup_engine: string;
  ai_cleanup_tone: string;
  input_device: string;
  injection_mode: string;
  vad_enabled: boolean;
  app_profiles: AppProfile[];
  onboarding_completed: boolean;
  theme: 'system' | 'light' | 'dark';
  mute_while_dictating: boolean;
  /** Greeting name. Empty = derive from the OS account (`os_display_name`). */
  display_name: string;
}

// Mirrors `Status` in src-tauri/src/license/mod.rs (serde tag = "kind").
export type LicenseStatus =
  | { kind: 'unlinked' }
  | { kind: 'active'; pubkey: string; balance: number; usd: number; ticker?: string }
  | { kind: 'grace'; pubkey: string; usd: number; days_left: number; ticker?: string }
  | { kind: 'locked'; pubkey: string; usd: number; ticker?: string };

export interface LicenseTokenQuote {
  mint: string;
  ticker: string;
  name: string;
  price_usd: number;
  tokens_required: number | null;
}

// Reply from the `license_floor` command.
export interface LicenseFloor {
  floor_usd: number;
  tokens?: LicenseTokenQuote[];
  price_usd?: number;
  tokens_required?: number | null;
  mint?: string;
}

/** Whether the gate currently permits dictation. Mirrors
 *  `Status::allows_dictation` in src-tauri/src/license/mod.rs. */
export function allowsDictation(status: LicenseStatus | null): boolean {
  return status?.kind === 'active' || status?.kind === 'grace';
}

/** "DuXu…bonk" — a Solana pubkey at a glance. */
export function shortAddress(pubkey: string): string {
  return pubkey.length > 12 ? `${pubkey.slice(0, 4)}…${pubkey.slice(-4)}` : pubkey;
}

export interface PanelProps {
  config: Config;
  update: (patch: Partial<Config>) => void;
}

import { platform } from '@tauri-apps/plugin-os';

// Typed-keystroke injection (SendInput) is Windows-only; on macOS every mode
// resolves to clipboard paste, so the option is hidden there.
// plugin-os reads injected metadata synchronously (navigator.platform is
// deprecated and lies inside some webviews).
export const IS_MAC = platform() === 'macos';

// KeyboardEvent.code → Lectus config key string.
// MUST stay in sync with config_key_to_vk in src-tauri/src/hook/mod.rs.
export const CODE_TO_KEY: Record<string, string> = {
  ControlRight: 'RControl',
  ControlLeft: 'LControl',
  ShiftRight: 'RShift',
  ShiftLeft: 'LShift',
  AltRight: 'RAlt',
  AltLeft: 'LAlt',
  MetaLeft: 'LWin',
  MetaRight: 'RWin',
  F13: 'F13',
  F14: 'F14',
  F15: 'F15',
};

// Config key string → human-readable label. macOS uses the standard modifier
// glyphs (⌃⇧⌥⌘, right-side variants annotated); Windows uses Fluent-style
// names. F-keys pass through unchanged on both.
const KEY_LABELS_MAC: Record<string, string> = {
  LControl: '⌃', RControl: '⌃ (right)',
  LShift: '⇧', RShift: '⇧ (right)',
  LAlt: '⌥', RAlt: '⌥ (right)',
  LWin: '⌘', RWin: '⌘ (right)',
};
const KEY_LABELS_WIN: Record<string, string> = {
  LControl: 'Ctrl', RControl: 'Right Ctrl',
  LShift: 'Shift', RShift: 'Right Shift',
  LAlt: 'Alt', RAlt: 'Right Alt',
  LWin: 'Win', RWin: 'Right Win',
};

/** Format a config hotkey string ("LControl+LWin") for display: "⌃⌘" on
 *  macOS, "Ctrl+Win" on Windows. Unknown keys (F13…) pass through. */
export function formatHotkey(config: string): string {
  if (!config) return config;
  const labels = IS_MAC ? KEY_LABELS_MAC : KEY_LABELS_WIN;
  const parts = config.split('+').map((k) => labels[k] ?? k);
  if (IS_MAC) {
    // Pure glyph chords concatenate Apple-style (⌃⌘); anything containing
    // text (F14, right-side annotations) keeps a separator for legibility.
    return parts.every((p) => p.length === 1) ? parts.join('') : parts.join(' ');
  }
  return parts.join('+');
}

// Common languages for the auto-detect override.
export const LANGUAGES: Array<{ code: string; label: string }> = [
  { code: 'auto', label: 'Auto-detect' },
  { code: 'en', label: 'English' },
  { code: 'pt', label: 'Portuguese' },
  { code: 'es', label: 'Spanish' },
  { code: 'fr', label: 'French' },
  { code: 'de', label: 'German' },
  { code: 'it', label: 'Italian' },
  { code: 'nl', label: 'Dutch' },
  { code: 'ja', label: 'Japanese' },
  { code: 'zh', label: 'Chinese' },
];
