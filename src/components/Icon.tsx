// Inline-SVG stroke icon set (SF-Symbols-like weight) replacing the old
// emoji nav glyphs — renders identically on both platforms and themes
// (uses currentColor). Paths are lifted from docs/mockups/settings.html.

export type IconName =
  | 'home'
  | 'general'
  | 'dictation'
  | 'vocabulary'
  | 'ai'
  | 'history'
  | 'models'
  | 'mic'
  | 'accessibility'
  | 'key'
  | 'help'
  | 'check';

const PATHS: Record<IconName, string> = {
  home: 'M3 11.5L12 4l9 7.5M5 10.5V20h14v-9.5',
  general:
    'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 15a1.7 1.7 0 0 0 .34 1.87l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.7 1.7 0 0 0-1.87-.34 1.7 1.7 0 0 0-1 1.55V21a2 2 0 1 1-4 0v-.09a1.7 1.7 0 0 0-1-1.55 1.7 1.7 0 0 0-1.87.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.7 1.7 0 0 0 .34-1.87 1.7 1.7 0 0 0-1.55-1H3a2 2 0 1 1 0-4h.09a1.7 1.7 0 0 0 1.55-1 1.7 1.7 0 0 0-.34-1.87l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.7 1.7 0 0 0 1.87.34h.09a1.7 1.7 0 0 0 1-1.55V3a2 2 0 1 1 4 0v.09a1.7 1.7 0 0 0 1 1.55 1.7 1.7 0 0 0 1.87-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.7 1.7 0 0 0-.34 1.87v.09a1.7 1.7 0 0 0 1.55 1H21a2 2 0 1 1 0 4h-.09a1.7 1.7 0 0 0-1.55 1z',
  dictation: 'M9 2h6v12H9zM9 2a3 3 0 0 1 6 0M5 10v2a7 7 0 0 0 14 0v-2M12 19v3',
  vocabulary: 'M4 19.5A2.5 2.5 0 0 1 6.5 17H20M4 19.5A2.5 2.5 0 0 0 6.5 22H20V2H6.5A2.5 2.5 0 0 0 4 4.5z',
  ai: 'M12 3l1.9 5.8L20 10l-6.1 1.2L12 17l-1.9-5.8L4 10l6.1-1.2zM19 17l.8 2.2L22 20l-2.2.8L19 23l-.8-2.2L16 20l2.2-.8z',
  history: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 7v5l3 3',
  models: 'M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16zM3.3 7l8.7 5 8.7-5M12 22V12',
  mic: 'M9 2h6v12H9zM9 2a3 3 0 0 1 6 0M5 10v2a7 7 0 0 0 14 0v-2M12 19v3',
  accessibility: 'M12 3a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM4 8h16M12 8v13M8 21l4-6 4 6M7 12h10',
  key: 'M15 7a4 4 0 1 1-4 4H3v3h2v3h3v-3h2v-2.17A4 4 0 0 1 15 7z',
  help: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM9.5 9.5a2.5 2.5 0 1 1 3.6 2.2c-.7.4-1.1 1-1.1 1.8v.5M12 17h.01',
  check: 'M20 6L9 17l-5-5',
};

export function Icon({ name, size = 16 }: { name: IconName; size?: number }) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      stroke="currentColor"
      fill="none"
      strokeWidth={1.6}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={PATHS[name]} />
    </svg>
  );
}
