import type {
  RecorderPreviewFrame,
  RecorderRecordingStats,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import { CircleDotDashed } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Card, CardContent } from '@/components/ui/card';
import { SceneCanvas } from './scene-canvas';
import { SceneControls } from './scene-controls';

export interface SceneCanvasPanelProps {
  configurationLocked?: boolean;
  onChange: (scene: RecorderScene) => void;
  onSelect: (sourceId: string | null) => void;
  previewFramesBySceneSourceId: Record<string, RecorderPreviewFrame>;
  recordingStats: RecorderRecordingStats | null;
  recordingStatsError: string | null;
  scene: RecorderScene | null;
  selectedSourceId: string | null;
  status: RecorderStatus | null;
}

export function SceneCanvasPanel({
  configurationLocked = false,
  onChange,
  onSelect,
  previewFramesBySceneSourceId,
  recordingStats,
  recordingStatsError,
  scene,
  selectedSourceId,
  status,
}: SceneCanvasPanelProps) {
  if (!scene) {
    return (
      <Card>
        <CardContent className="text-muted-foreground text-sm">
          Loading recorder workspace.
        </CardContent>
      </Card>
    );
  }

  if (status?.state === 'recording') {
    return (
      <RecordingInProgressPanel
        recordingStats={recordingStats}
        recordingStatsError={recordingStatsError}
      />
    );
  }

  return (
    <div className="flex h-full w-full flex-col">
      <SceneControls
        disabled={configurationLocked}
        scene={scene}
        onChange={onChange}
      />
      <div className="min-h-0 flex-1 p-4">
        <SceneCanvas
          previewFramesBySceneSourceId={previewFramesBySceneSourceId}
          readOnly={configurationLocked}
          scene={scene}
          selectedSourceId={selectedSourceId}
          onChange={onChange}
          onSelect={onSelect}
        />
      </div>
    </div>
  );
}

function RecordingInProgressPanel({
  recordingStats,
  recordingStatsError,
}: {
  recordingStats: RecorderRecordingStats | null;
  recordingStatsError: string | null;
}) {
  return (
    <div className="flex h-full min-h-[320px] w-full items-center justify-center p-6">
      <section
        aria-label="Recording in progress"
        className="bg-background w-full max-w-2xl border p-6 shadow-sm"
      >
        <div className="flex items-start gap-3">
          <span className="bg-destructive/10 text-destructive flex size-10 shrink-0 items-center justify-center rounded-full">
            <CircleDotDashed className="size-5" aria-hidden="true" />
          </span>
          <div className="min-w-0">
            <h2 className="text-lg font-semibold tracking-normal">
              Recording in progress
            </h2>
            <p className="text-muted-foreground mt-1 text-sm">
              Preview is disabled while capture is writing local segment files.
            </p>
          </div>
        </div>

        <dl className="mt-6 grid gap-3 sm:grid-cols-2">
          <RecordingStat
            label="Segment"
            value={formatSegment(recordingStats)}
          />
          <RecordingStat
            label="Segment time"
            value={formatSegmentDuration(recordingStats)}
          />
          <RecordingStat
            label="Segment size"
            value={formatOptionalSize(recordingStats?.activeSegmentSizeBytes)}
          />
          <RecordingStat
            label="Total size"
            value={formatRequiredSize(recordingStats?.totalSizeBytes)}
          />
          <RecordingStat
            className="sm:col-span-2"
            label="Recording started"
            value={formatTimestamp(recordingStats?.recordingStartedAtMs)}
          />
        </dl>

        {recordingStatsError ? (
          <Alert className="mt-6" variant="destructive">
            <AlertTitle>Stats unavailable</AlertTitle>
            <AlertDescription>{recordingStatsError}</AlertDescription>
          </Alert>
        ) : null}
      </section>
    </div>
  );
}

function RecordingStat({
  className,
  label,
  value,
}: {
  className?: string;
  label: string;
  value: string;
}) {
  return (
    <div className={className}>
      <dt className="text-muted-foreground text-xs font-medium tracking-normal uppercase">
        {label}
      </dt>
      <dd className="mt-1 text-sm font-medium tabular-nums">{value}</dd>
    </div>
  );
}

function formatSegment(stats: RecorderRecordingStats | null): string {
  if (
    stats?.activeSegmentIndex === undefined ||
    stats.activeSegmentIndex === null
  ) {
    return 'Calculating';
  }

  return `${stats.activeSegmentIndex} of ${Math.max(
    stats.segmentCount,
    stats.activeSegmentIndex,
  )}`;
}

function formatSegmentDuration(stats: RecorderRecordingStats | null): string {
  if (
    stats?.activeSegmentStartedAtMs === undefined ||
    stats.activeSegmentStartedAtMs === null
  ) {
    return 'Calculating';
  }

  return formatDuration(
    Math.max(0, stats.updatedAtMs - stats.activeSegmentStartedAtMs),
  );
}

function formatDuration(durationMs: number): string {
  const totalSeconds = Math.floor(durationMs / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}:${padTime(minutes)}:${padTime(seconds)}`;
  }

  return `${minutes}:${padTime(seconds)}`;
}

function formatOptionalSize(sizeBytes: number | null | undefined): string {
  if (sizeBytes === null || sizeBytes === undefined) {
    return 'Calculating';
  }

  return formatRequiredSize(sizeBytes);
}

function formatRequiredSize(sizeBytes: number | null | undefined): string {
  if (sizeBytes === null || sizeBytes === undefined) {
    return 'Calculating';
  }

  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  let value = Math.max(0, sizeBytes);
  let unitIndex = 0;
  while (value >= 1000 && unitIndex < units.length - 1) {
    value /= 1000;
    unitIndex += 1;
  }

  if (unitIndex === 0) {
    return `${value} ${units[unitIndex]}`;
  }

  return `${value >= 10 ? value.toFixed(1) : value.toFixed(2)} ${
    units[unitIndex]
  }`;
}

function formatTimestamp(timestampMs: number | null | undefined): string {
  if (timestampMs === null || timestampMs === undefined) {
    return 'Calculating';
  }

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(timestampMs));
}

function padTime(value: number): string {
  return value.toString().padStart(2, '0');
}
