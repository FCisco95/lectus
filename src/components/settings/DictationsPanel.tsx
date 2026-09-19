import type { Config } from './types';
import { HistoryList } from './HistoryList';

interface DictationsPanelProps {
  config: Config;
}

/** Dedicated Dictations surface — same list Home shows under the KPIs. */
export function DictationsPanel({ config }: DictationsPanelProps) {
  return <HistoryList config={config} />;
}
