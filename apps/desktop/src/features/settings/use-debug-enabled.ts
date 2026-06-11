import { useEffect } from 'react';
import { useSettingsStore } from './settings-store';

export function useDebugEnabled(): boolean {
  const draft = useSettingsStore((state) => state.draft);
  const load = useSettingsStore((state) => state.load);
  const loadStatus = useSettingsStore((state) => state.loadStatus);
  const settingsState = useSettingsStore((state) => state.settingsState);

  useEffect(() => {
    if (loadStatus === 'idle') {
      void load();
    }
  }, [load, loadStatus]);

  return (
    draft?.debug?.enabled ?? settingsState?.settings.debug?.enabled ?? false
  );
}
