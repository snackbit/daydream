import type { AppSettings, AppSettingsLimits } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import type { UpdateSettingsDraft } from '../../settings-draft-updaters';
import { GIB_BYTES, gibToBytes } from '../../settings-format';
import { NumberPreference } from '../controls/number-preference';
import { SettingRow } from '../controls/setting-row';

export interface StoragePreferencesProps {
  draft: AppSettings;
  limits: AppSettingsLimits;
  onUpdate: UpdateSettingsDraft;
}
export function StoragePreferences({
  draft,
  limits,
  onUpdate,
}: StoragePreferencesProps) {
  function retention(value: number) {
    onUpdate((current) => ({
      ...current,
      storage: { ...current.storage, retentionDays: Math.round(value) },
    }));
  }
  function size(value: number) {
    onUpdate((current) => ({
      ...current,
      storage: { ...current.storage, sizeLimitBytes: gibToBytes(value) },
    }));
  }
  return (
    <div>
      <SettingRow
        title="Keep recordings for"
        description="Review affected recordings before shortening retention."
      >
        <NumberPreference
          label="Recording retention"
          value={draft.storage.retentionDays}
          min={limits.storage.retentionDaysMin}
          max={limits.storage.retentionDaysMax}
          unit="days"
          onCommit={retention}
        />
      </SettingRow>
      <SettingRow
        title="Review storage policy"
        description="Review cleanup before applying this policy to your library."
      >
        <Button
          variant="outline"
          onClick={() => onUpdate((current) => ({ ...current }))}
        >
          Review policy
        </Button>
      </SettingRow>
      <SettingRow
        title="Library size limit"
        description="Older recordings are removed when the library reaches this limit."
      >
        <NumberPreference
          label="Storage size limit"
          value={draft.storage.sizeLimitBytes / GIB_BYTES}
          min={limits.storage.sizeLimitBytesMin / GIB_BYTES}
          unit="GiB"
          step={0.1}
          onCommit={size}
        />
      </SettingRow>
    </div>
  );
}
