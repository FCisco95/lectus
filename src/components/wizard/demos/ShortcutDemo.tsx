import { formatHotkey } from '../../settings/types';
export function ShortcutDemo({ shortcut, toggle }: { shortcut: string; toggle: boolean }) {
  return <div className="wizard-demo" aria-label="Demo: dictate with your shortcut"><span className="kbd">{formatHotkey(shortcut)}</span>
    <div className="demo-message"><span className="demo-typed">A note, without typing.</span></div><p>{toggle ? 'Tap to start. Tap to finish.' : 'Hold to speak. Release to type.'}</p></div>;
}
