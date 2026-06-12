import type { AppSettings, AppSettingsLimits } from '@daydream/shared';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { FieldGroup } from '@/components/ui/field';
import { formatFps } from '@/features/settings/settings-format';
import { CheckboxField, SliderField } from './ai-settings-fields';

interface AiActivityProcessingCardProps {
  draft: AppSettings;
  limits: AppSettingsLimits;
  onUpdateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
}

export function AiActivityProcessingCard({
  draft,
  limits,
  onUpdateDraft,
}: AiActivityProcessingCardProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Activity and Processing</CardTitle>
        <CardDescription>
          Controls for automatic processing and activity inference.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <FieldGroup>
          <CheckboxField
            checked={draft.ai.processing.autoStartAfterRecording}
            description="Process eligible, non-redacted recordings after capture ends."
            label="Start processing after recording"
            onCheckedChange={(checked) => {
              onUpdateDraft((current) => ({
                ...current,
                ai: {
                  ...current.ai,
                  processing: {
                    ...current.ai.processing,
                    autoStartAfterRecording: checked,
                  },
                },
              }));
            }}
          />
          <CheckboxField
            checked={draft.ai.processing.frameSimilarityEnabled}
            description="Skip visually similar frame regions between periodic full-frame passes."
            label="Use frame similarity cache"
            onCheckedChange={(checked) => {
              onUpdateDraft((current) => ({
                ...current,
                ai: {
                  ...current.ai,
                  processing: {
                    ...current.ai.processing,
                    frameSimilarityEnabled: checked,
                  },
                },
              }));
            }}
          />
          <CheckboxField
            checked={draft.ai.processing.frameRecognitionSyncWithRecording}
            description="Use the recording frame cadence for OCR and VLM frame recognition."
            label="Sync frame recognition with recording FPS"
            onCheckedChange={(checked) => {
              onUpdateDraft((current) => ({
                ...current,
                ai: {
                  ...current.ai,
                  processing: {
                    ...current.ai.processing,
                    frameRecognitionSyncWithRecording: checked,
                  },
                },
              }));
            }}
          />
          <div className="grid gap-4 md:grid-cols-3">
            <SliderField
              label="Parallel jobs"
              max={limits.ai.processing.maxParallelJobsMax}
              min={limits.ai.processing.maxParallelJobsMin}
              onChange={(value) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    processing: {
                      ...current.ai.processing,
                      maxParallelJobs: value,
                    },
                  },
                }));
              }}
              value={draft.ai.processing.maxParallelJobs}
            />
            <SliderField
              label="Chunks per job"
              max={limits.ai.processing.maxParallelChunksPerJobMax}
              min={limits.ai.processing.maxParallelChunksPerJobMin}
              onChange={(value) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    processing: {
                      ...current.ai.processing,
                      maxParallelChunksPerJob: value,
                    },
                  },
                }));
              }}
              value={draft.ai.processing.maxParallelChunksPerJob}
            />
            <SliderField
              label="Frame recognition FPS"
              max={limits.ai.processing.frameRecognitionFpsMax}
              min={limits.ai.processing.frameRecognitionFpsMin}
              formatValue={formatFps}
              onChange={(value) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    processing: {
                      ...current.ai.processing,
                      frameRecognitionFps: value,
                    },
                  },
                }));
              }}
              rangeLabel={`${formatFps(
                limits.ai.processing.frameRecognitionFpsMin,
              )} to ${formatFps(limits.ai.processing.frameRecognitionFpsMax)}`}
              step={0.05}
              stepLabel="0.05 fps"
              value={draft.ai.processing.frameRecognitionFps}
            />
            <SliderField
              label="Full-frame interval"
              max={limits.ai.processing.periodicFullFrameIntervalMsMax}
              min={limits.ai.processing.periodicFullFrameIntervalMsMin}
              formatValue={formatDuration}
              onChange={(value) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    processing: {
                      ...current.ai.processing,
                      periodicFullFrameIntervalMs: value,
                    },
                  },
                }));
              }}
              rangeLabel={`${formatDuration(
                limits.ai.processing.periodicFullFrameIntervalMsMin,
              )} to ${formatDuration(
                limits.ai.processing.periodicFullFrameIntervalMsMax,
              )}`}
              step={500}
              stepLabel="0.5 seconds"
              value={draft.ai.processing.periodicFullFrameIntervalMs}
            />
            <SliderField
              label="Full-frame cap"
              max={limits.ai.processing.maxFullFrameCheckpointsPerMinuteMax}
              min={limits.ai.processing.maxFullFrameCheckpointsPerMinuteMin}
              onChange={(value) => {
                onUpdateDraft((current) => ({
                  ...current,
                  ai: {
                    ...current.ai,
                    processing: {
                      ...current.ai.processing,
                      maxFullFrameCheckpointsPerMinute: value,
                    },
                  },
                }));
              }}
              suffix="/min"
              value={draft.ai.processing.maxFullFrameCheckpointsPerMinute}
            />
            {draft.expert?.enabled ? (
              <SliderField
                label="Delta tile size"
                max={limits.ai.processing.visualDeltaTileSizePxMax}
                min={limits.ai.processing.visualDeltaTileSizePxMin}
                onChange={(value) => {
                  onUpdateDraft((current) => ({
                    ...current,
                    ai: {
                      ...current.ai,
                      processing: {
                        ...current.ai.processing,
                        visualDeltaTileSizePx: value,
                      },
                    },
                  }));
                }}
                step={16}
                suffix="px"
                value={draft.ai.processing.visualDeltaTileSizePx}
              />
            ) : null}
          </div>
          <CheckboxField
            checked={draft.ai.activity.keyboard.enabled}
            description="Linux uses privileged /dev/input timing only; key values and typed text are not stored."
            label="Use keyboard activity"
            onCheckedChange={(checked) => {
              onUpdateDraft((current) => ({
                ...current,
                ai: {
                  ...current.ai,
                  activity: {
                    ...current.ai.activity,
                    keyboard: {
                      ...current.ai.activity.keyboard,
                      enabled: checked,
                    },
                  },
                },
              }));
            }}
          />
          <SliderField
            label="Keyboard activity window"
            max={limits.ai.activity.keyboardWindowMsMax}
            min={limits.ai.activity.keyboardWindowMsMin}
            onChange={(value) => {
              onUpdateDraft((current) => ({
                ...current,
                ai: {
                  ...current.ai,
                  activity: {
                    ...current.ai.activity,
                    keyboard: {
                      ...current.ai.activity.keyboard,
                      windowMs: value,
                    },
                  },
                },
              }));
            }}
            step={1_000}
            suffix="ms"
            value={draft.ai.activity.keyboard.windowMs}
          />
          <CheckboxField
            checked={draft.ai.activity.mouse.enabled}
            description="Linux uses privileged /dev/input timing only; coordinates and target content are not stored."
            label="Use mouse activity"
            onCheckedChange={(checked) => {
              onUpdateDraft((current) => ({
                ...current,
                ai: {
                  ...current.ai,
                  activity: {
                    ...current.ai.activity,
                    mouse: {
                      ...current.ai.activity.mouse,
                      enabled: checked,
                    },
                  },
                },
              }));
            }}
          />
          <SliderField
            label="Mouse activity window"
            max={limits.ai.activity.mouseWindowMsMax}
            min={limits.ai.activity.mouseWindowMsMin}
            onChange={(value) => {
              onUpdateDraft((current) => ({
                ...current,
                ai: {
                  ...current.ai,
                  activity: {
                    ...current.ai.activity,
                    mouse: {
                      ...current.ai.activity.mouse,
                      windowMs: value,
                    },
                  },
                },
              }));
            }}
            step={1_000}
            suffix="ms"
            value={draft.ai.activity.mouse.windowMs}
          />
        </FieldGroup>
      </CardContent>
    </Card>
  );
}

function formatDuration(valueMs: number): string {
  if (valueMs >= 60_000 && valueMs % 60_000 === 0) {
    const minutes = valueMs / 60_000;
    return `${minutes} ${minutes === 1 ? 'minute' : 'minutes'}`;
  }
  if (valueMs >= 1_000) {
    const seconds = valueMs / 1_000;
    const formattedSeconds = Number.isInteger(seconds)
      ? seconds.toString()
      : seconds.toFixed(1).replace(/\.?0+$/, '');

    return `${formattedSeconds} ${seconds === 1 ? 'second' : 'seconds'}`;
  }

  return `${valueMs} ms`;
}
