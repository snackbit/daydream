import type {
  AiProcessingSettings,
  AiProcessingLimits,
  AppSettings,
  AppSettingsLimits,
} from '@daydream/shared';
import { Switch } from '@/components/ui/switch';
import type { UpdateSettingsDraft } from '../../settings-draft-updaters';
import { NumberPreference } from '../controls/number-preference';
import { SettingRow } from '../controls/setting-row';

const numbers = [
  {
    key: 'maxParallelJobs',
    label: 'Parallel jobs',
    min: 'maxParallelJobsMin',
    max: 'maxParallelJobsMax',
  },
  {
    key: 'maxParallelChunksPerJob',
    label: 'Chunks per job',
    min: 'maxParallelChunksPerJobMin',
    max: 'maxParallelChunksPerJobMax',
  },
  {
    key: 'maxParallelOcrJobs',
    label: 'Text recognition jobs',
    min: 'maxParallelOcrJobsMin',
    max: 'maxParallelOcrJobsMax',
  },
  {
    key: 'maxParallelVlmJobs',
    label: 'Screen observation jobs',
    min: 'maxParallelVlmJobsMin',
    max: 'maxParallelVlmJobsMax',
  },
  {
    key: 'frameRecognitionFps',
    label: 'Screen processing rate',
    min: 'frameRecognitionFpsMin',
    max: 'frameRecognitionFpsMax',
    unit: 'fps',
    step: 0.05,
  },
  {
    key: 'periodicFullFrameIntervalMs',
    label: 'Full frame interval',
    min: 'periodicFullFrameIntervalMsMin',
    max: 'periodicFullFrameIntervalMsMax',
    unit: 'ms',
  },
  {
    key: 'maxFullFrameCheckpointsPerMinute',
    label: 'Full frames per minute',
    min: 'maxFullFrameCheckpointsPerMinuteMin',
    max: 'maxFullFrameCheckpointsPerMinuteMax',
  },
  {
    key: 'visualDeltaTileSizePx',
    label: 'Change detection tile size',
    min: 'visualDeltaTileSizePxMin',
    max: 'visualDeltaTileSizePxMax',
    unit: 'px',
    step: 16,
  },
] satisfies {
  key: keyof AiProcessingSettings;
  label: string;
  min: keyof AiProcessingLimits;
  max: keyof AiProcessingLimits;
  unit?: string;
  step?: number;
}[];
function ProcessingNumber({
  item,
  draft,
  limits,
  onUpdate,
}: {
  item: (typeof numbers)[number];
  draft: AppSettings;
  limits: AppSettingsLimits;
  onUpdate: UpdateSettingsDraft;
}) {
  function change(value: number) {
    onUpdate((current) => ({
      ...current,
      ai: {
        ...current.ai,
        processing: { ...current.ai.processing, [item.key]: value },
      },
    }));
  }
  return (
    <SettingRow title={item.label}>
      <NumberPreference
        label={item.label}
        value={draft.ai.processing[item.key]}
        min={limits.ai.processing[item.min]}
        max={limits.ai.processing[item.max]}
        unit={item.unit}
        step={item.step}
        onCommit={change}
      />
    </SettingRow>
  );
}
export function ProcessingPreferences({
  draft,
  limits,
  onUpdate,
}: {
  draft: AppSettings;
  limits: AppSettingsLimits;
  onUpdate: UpdateSettingsDraft;
}) {
  function sync(frameRecognitionSyncWithRecording: boolean) {
    onUpdate((current) => ({
      ...current,
      ai: {
        ...current.ai,
        processing: {
          ...current.ai.processing,
          frameRecognitionSyncWithRecording,
        },
      },
    }));
  }
  function similarity(frameSimilarityEnabled: boolean) {
    onUpdate((current) => ({
      ...current,
      ai: {
        ...current.ai,
        processing: { ...current.ai.processing, frameSimilarityEnabled },
      },
    }));
  }
  function deltas(enabled: boolean) {
    onUpdate((current) => ({
      ...current,
      ai: {
        ...current.ai,
        activity: { ...current.ai.activity, frameDeltas: { enabled } },
      },
    }));
  }
  return (
    <div>
      <SettingRow title="Match recording cadence">
        <Switch
          aria-label="Match recording cadence"
          checked={draft.ai.processing.frameRecognitionSyncWithRecording}
          onCheckedChange={sync}
        />
      </SettingRow>
      <SettingRow title="Skip similar frames">
        <Switch
          aria-label="Skip similar frames"
          checked={draft.ai.processing.frameSimilarityEnabled}
          onCheckedChange={similarity}
        />
      </SettingRow>
      <SettingRow title="Track screen changes">
        <Switch
          aria-label="Track screen changes"
          checked={draft.ai.activity.frameDeltas.enabled}
          onCheckedChange={deltas}
        />
      </SettingRow>
      {numbers.map((item) => (
        <ProcessingNumber
          key={item.key}
          item={item}
          draft={draft}
          limits={limits}
          onUpdate={onUpdate}
        />
      ))}
    </div>
  );
}
