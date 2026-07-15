// Mirrors `Config` / `ReplacementRule` in src-tauri/src/config.rs.

export interface ReplacementRule {
  from: string;
  to: string;
  case_sensitive: boolean;
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
}

export interface PanelProps {
  config: Config;
  update: (patch: Partial<Config>) => void;
}

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
