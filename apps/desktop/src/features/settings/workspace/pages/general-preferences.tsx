import type { AppSettings } from '@daydream/shared';
import { Switch } from '@/components/ui/switch';
import { AppearanceControl } from '@/features/appearance/components/appearance-control';
import type { UpdateSettingsDraft } from '../../settings-draft-updaters';
import { SettingRow } from '../controls/setting-row';

export interface GeneralPreferencesProps {
  draft: AppSettings;
  onUpdate: UpdateSettingsDraft;
}
export function GeneralPreferences({
  draft,
  onUpdate,
}: GeneralPreferencesProps) {
  function expert(enabled: boolean) {
    onUpdate((current) => ({
      ...current,
      expert: { ...current.expert, enabled },
    }));
  }
  function debug(enabled: boolean) {
    onUpdate((current) => ({
      ...current,
      debug: { ...current.debug, enabled },
    }));
  }
  return (
    <div>
      <SettingRow title="Appearance">
        <AppearanceControl />
      </SettingRow>
      <SettingRow
        title="Advanced controls"
        description="Show processing and model tuning options."
      >
        <Switch
          aria-label="Enable expert mode"
          checked={draft.expert?.enabled ?? false}
          onCheckedChange={expert}
        />
      </SettingRow>
      <SettingRow
        title="Debug tools"
        description="Show Diagnostics and recording debug data."
      >
        <Switch
          aria-label="Enable debug tools"
          checked={draft.debug?.enabled ?? false}
          onCheckedChange={debug}
        />
      </SettingRow>
    </div>
  );
}
