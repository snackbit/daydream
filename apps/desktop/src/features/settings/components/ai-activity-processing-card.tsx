import type {
  AppSettings,
  AppSettingsLimits,
  InputActivityStatus,
} from '@daydream/shared';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { FieldGroup } from '@/components/ui/field';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';
import {
  updateAiKeyboardActivity,
  updateAiMouseActivity,
  updateAiProcessing,
  updateAiWindowActivity,
} from '@/features/settings/settings-draft-updaters';
import { formatFps } from '@/features/settings/settings-format';
import { CheckboxField, SliderField } from './ai-settings-fields';
import { InputActivityStatusLine } from './input-activity-status-line';

interface AiActivityProcessingCardProps {
  draft: AppSettings;
  inputActivityRepairMessage: string | null;
  inputActivityRepairing: boolean;
  inputActivityStatus: InputActivityStatus | null;
  limits: AppSettingsLimits;
  onRefreshInputActivityStatus: () => void;
  onRepairInputActivity: () => void;
  onUpdateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
}

export function AiActivityProcessingCard({
  draft,
  inputActivityRepairMessage,
  inputActivityRepairing,
  inputActivityStatus,
  limits,
  onRefreshInputActivityStatus,
  onRepairInputActivity,
  onUpdateDraft,
}: AiActivityProcessingCardProps) {
  function setTiming(value: string) {
    if (
      value !== 'during-recording' &&
      value !== 'after-stop' &&
      value !== 'manual'
    )
      return;
    updateAiProcessing(onUpdateDraft, (current) => ({
      ...current,
      timing: value,
      autoStartAfterRecording: value !== 'manual',
    }));
  }
  function setResourceMode(value: string) {
    if (value !== 'background' && value !== 'fast') return;
    updateAiProcessing(onUpdateDraft, (current) => ({
      ...current,
      resourceMode: value,
    }));
  }
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Activity and Processing</CardTitle>
        <CardDescription>
          Controls for automatic processing and activity inference.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <FieldGroup>
          <div className="grid gap-2 md:grid-cols-2">
            <div className="space-y-1">
              <span className="text-xs font-medium">Processing timing</span>
              <Select
                value={
                  draft.ai.processing.autoStartAfterRecording
                    ? (draft.ai.processing.timing ?? 'after-stop')
                    : 'manual'
                }
                onValueChange={setTiming}
              >
                <SelectTrigger aria-label="Processing timing">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="during-recording">
                    During recording
                  </SelectItem>
                  <SelectItem value="after-stop">After stop</SelectItem>
                  <SelectItem value="manual">Manual</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1">
              <span className="text-xs font-medium">Resource use</span>
              <Select
                value={draft.ai.processing.resourceMode ?? 'background'}
                onValueChange={setResourceMode}
              >
                <SelectTrigger aria-label="Processing resource use">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="background">Background</SelectItem>
                  <SelectItem value="fast">Fast</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <p className="text-muted-foreground text-xs">
            During recording processes finalized sections once their privacy
            checks are complete. Background limits competing work; Fast uses the
            configured concurrency limits.
          </p>
          <CheckboxField
            checked={draft.ai.processing.frameSimilarityEnabled}
            description="Reduce duplicate or visually unchanged work in screen observations. Vision OCR still checks every selected full frame and reuses results only for exact image matches."
            label="Optimize screen observations"
            onCheckedChange={(checked) => {
              updateAiProcessing(onUpdateDraft, (current) => ({
                ...current,
                frameSimilarityEnabled: checked,
              }));
            }}
          />
          <CheckboxField
            checked={draft.ai.processing.frameRecognitionSyncWithRecording}
            description="Use the recording frame cadence for OCR and VLM frame recognition."
            label="Sync frame recognition with recording FPS"
            onCheckedChange={(checked) => {
              updateAiProcessing(onUpdateDraft, (current) => ({
                ...current,
                frameRecognitionSyncWithRecording: checked,
              }));
            }}
          />
          <div className="grid gap-4 md:grid-cols-3">
            <SliderField
              label="Parallel jobs"
              max={limits.ai.processing.maxParallelJobsMax}
              min={limits.ai.processing.maxParallelJobsMin}
              onChange={(value) => {
                updateAiProcessing(onUpdateDraft, (current) => ({
                  ...current,
                  maxParallelJobs: value,
                }));
              }}
              value={draft.ai.processing.maxParallelJobs}
            />
            <SliderField
              label="Chunks per job"
              max={limits.ai.processing.maxParallelChunksPerJobMax}
              min={limits.ai.processing.maxParallelChunksPerJobMin}
              onChange={(value) => {
                updateAiProcessing(onUpdateDraft, (current) => ({
                  ...current,
                  maxParallelChunksPerJob: value,
                }));
              }}
              value={draft.ai.processing.maxParallelChunksPerJob}
            />
            <SliderField
              label="OCR jobs"
              max={limits.ai.processing.maxParallelOcrJobsMax}
              min={limits.ai.processing.maxParallelOcrJobsMin}
              onChange={(value) => {
                updateAiProcessing(onUpdateDraft, (current) => ({
                  ...current,
                  maxParallelOcrJobs: value,
                }));
              }}
              value={draft.ai.processing.maxParallelOcrJobs}
            />
            <SliderField
              label="VLM jobs"
              max={limits.ai.processing.maxParallelVlmJobsMax}
              min={limits.ai.processing.maxParallelVlmJobsMin}
              onChange={(value) => {
                updateAiProcessing(onUpdateDraft, (current) => ({
                  ...current,
                  maxParallelVlmJobs: value,
                }));
              }}
              value={draft.ai.processing.maxParallelVlmJobs}
            />
            <SliderField
              label="Frame recognition FPS"
              max={limits.ai.processing.frameRecognitionFpsMax}
              min={limits.ai.processing.frameRecognitionFpsMin}
              formatValue={formatFps}
              onChange={(value) => {
                updateAiProcessing(onUpdateDraft, (current) => ({
                  ...current,
                  frameRecognitionFps: value,
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
                updateAiProcessing(onUpdateDraft, (current) => ({
                  ...current,
                  periodicFullFrameIntervalMs: value,
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
                updateAiProcessing(onUpdateDraft, (current) => ({
                  ...current,
                  maxFullFrameCheckpointsPerMinute: value,
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
                  updateAiProcessing(onUpdateDraft, (current) => ({
                    ...current,
                    visualDeltaTileSizePx: value,
                  }));
                }}
                step={16}
                suffix="px"
                value={draft.ai.processing.visualDeltaTileSizePx}
              />
            ) : null}
          </div>
          <CheckboxField
            checked={draft.ai.activity.windows.enabled}
            description="Capture focused-window changes and open-window inventory changes for the Activity lane."
            label="Use window activity"
            onCheckedChange={(checked) => {
              updateAiWindowActivity(onUpdateDraft, (current) => ({
                ...current,
                enabled: checked,
              }));
            }}
          />
          <CheckboxField
            checked={draft.ai.activity.keyboard.enabled}
            description="Linux uses privileged /dev/input timing only; key values and typed text are not stored."
            label="Use keyboard activity"
            onCheckedChange={(checked) => {
              updateAiKeyboardActivity(onUpdateDraft, (current) => ({
                ...current,
                enabled: checked,
              }));
            }}
          />
          <InputActivityStatusLine
            onRefresh={onRefreshInputActivityStatus}
            onRepair={onRepairInputActivity}
            repairMessage={inputActivityRepairMessage}
            repairing={inputActivityRepairing}
            status={inputActivityStatus}
          />
          <SliderField
            label="Keyboard activity window"
            max={limits.ai.activity.keyboardWindowMsMax}
            min={limits.ai.activity.keyboardWindowMsMin}
            onChange={(value) => {
              updateAiKeyboardActivity(onUpdateDraft, (current) => ({
                ...current,
                windowMs: value,
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
              updateAiMouseActivity(onUpdateDraft, (current) => ({
                ...current,
                enabled: checked,
              }));
            }}
          />
          <SliderField
            label="Mouse activity window"
            max={limits.ai.activity.mouseWindowMsMax}
            min={limits.ai.activity.mouseWindowMsMin}
            onChange={(value) => {
              updateAiMouseActivity(onUpdateDraft, (current) => ({
                ...current,
                windowMs: value,
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
