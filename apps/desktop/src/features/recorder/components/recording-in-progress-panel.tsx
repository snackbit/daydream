import type { RecorderRecordingStats } from '@daydream/shared';
import { CircleDotDashed } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';

export function RecordingInProgressPanel({
  recordingStats,
  recordingStatsError,
}: {
  recordingStats: RecorderRecordingStats | null;
  recordingStatsError: string | null;
}) {
  return (
    <section
      aria-label="Recording in progress"
      className="flex h-full min-h-[16rem] w-full flex-col"
    >
      <div className="flex items-center gap-2 border-b px-3 py-2">
        <CircleDotDashed
          className="text-destructive size-4 shrink-0"
          aria-hidden="true"
        />
        <div className="min-w-0">
          <h2 className="text-xs font-semibold">Recording in progress</h2>
          <p className="text-muted-foreground text-[10px]">
            Preview pauses while local segments are written.
          </p>
        </div>
      </div>

      <dl className="text-foreground flex flex-wrap gap-x-5 gap-y-2 px-3 py-2 text-xs">
        <RecordingStat label="Segment" value={formatSegment(recordingStats)} />
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
          className="basis-full sm:basis-auto"
          label="Started"
          value={formatTimestamp(recordingStats?.recordingStartedAtMs)}
        />
      </dl>

      {recordingStatsError ? (
        <Alert className="m-2" variant="destructive">
          <AlertTitle>Stats unavailable</AlertTitle>
          <AlertDescription>{recordingStatsError}</AlertDescription>
        </Alert>
      ) : null}
    </section>
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
    <div className={`flex min-w-0 items-baseline gap-1.5 ${className ?? ''}`}>
      <dt className="text-muted-foreground shrink-0 text-[10px]">{label}</dt>
      <dd className="min-w-0 text-[11px] font-medium tabular-nums">{value}</dd>
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
