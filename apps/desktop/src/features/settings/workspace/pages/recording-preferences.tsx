import { useState } from 'react';
import type {
  AppSettings,
  AppSettingsLimits,
  RecorderScene,
} from '@daydream/shared';
import { SharedDisclosure } from '@/components/shared-disclosure';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { UpdateSettingsDraft } from '../../settings-draft-updaters';
import {
  estimateOneHourRecordingBytes,
  formatSceneDimensions,
  formatStorageSize,
} from '../../settings-format';
import { NumberPreference } from '../controls/number-preference';
import { SettingRow } from '../controls/setting-row';

export interface RecordingPreferencesProps {
  draft: AppSettings;
  limits: AppSettingsLimits;
  scene: RecorderScene | null;
  onUpdate: UpdateSettingsDraft;
}
export function RecordingPreferences({
  draft,
  limits,
  scene,
  onUpdate,
}: RecordingPreferencesProps) {
  const [unit, setUnit] = useState(draft.record.fps < 1 ? 'seconds' : 'fps');
  function changeUnit(value: string) {
    if (value === 'seconds' || value === 'fps') setUnit(value);
  }
  function cadence(value: number) {
    onUpdate((current) => ({
      ...current,
      record: {
        ...current.record,
        fps: unit === 'seconds' ? 1 / value : value,
      },
    }));
  }
  function bitrate(value: number) {
    onUpdate((current) => ({
      ...current,
      record: { ...current.record, videoBitrateKbps: value },
    }));
  }
  const interval = unit === 'seconds' ? 1 / draft.record.fps : draft.record.fps;
  return (
    <div>
      <SettingRow
        title="Capture interval"
        description="Applies to new recordings."
      >
        <div className="flex items-start gap-2">
          <NumberPreference
            label={
              unit === 'seconds' ? 'Seconds per frame' : 'Frames per second'
            }
            value={Number(interval.toFixed(3))}
            min={
              unit === 'seconds'
                ? 1 / limits.record.fpsMax
                : limits.record.fpsMin
            }
            max={
              unit === 'seconds'
                ? 1 / limits.record.fpsMin
                : limits.record.fpsMax
            }
            step={0.1}
            onCommit={cadence}
          />
          <Select value={unit} onValueChange={changeUnit}>
            <SelectTrigger aria-label="Capture interval unit" className="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="seconds">seconds per frame</SelectItem>
              <SelectItem value="fps">frames per second</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </SettingRow>
      <SettingRow
        title="Recording estimate"
        description={formatSceneDimensions(scene)}
      >
        <span className="text-muted-foreground text-sm">
          About {formatStorageSize(estimateOneHourRecordingBytes(draft, scene))}{' '}
          / hour
        </span>
      </SettingRow>
      <div className="pt-5">
        <SharedDisclosure label="Advanced">
          <SettingRow title="Video bitrate">
            <NumberPreference
              label="Video bitrate"
              value={draft.record.videoBitrateKbps}
              min={limits.record.videoBitrateKbpsMin}
              max={limits.record.videoBitrateKbpsMax}
              step={250}
              unit="kbps"
              onCommit={bitrate}
            />
          </SettingRow>
        </SharedDisclosure>
      </div>
    </div>
  );
}
