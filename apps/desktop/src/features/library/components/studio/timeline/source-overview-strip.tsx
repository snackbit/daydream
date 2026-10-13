import type { StudioTimelineBucket } from '@daydream/shared';
import {
  groupTimelineEntries,
  TimelineEntryGroupButton,
} from './timeline-entry-groups';
import {
  TranscriptGroupPreview,
  type TranscriptPreviewReader,
} from './transcripts/transcript-group-preview';

export interface SourceOverviewStripProps {
  buckets: readonly StudioTimelineBucket[];
  trackId: string;
  width: number;
  duration: number;
  onZoom: (start: number, end: number) => void;
  range?: { startTimelineMs: number; endTimelineMs: number } | undefined;
  recordingId?: string | undefined;
  sourceRole?: 'desktop-audio' | 'microphone' | undefined;
  trackLabel?: string | undefined;
  readPreview?: TranscriptPreviewReader | undefined;
  previewRevision?: string | undefined;
  pending?: boolean | undefined;
  error?: boolean | undefined;
}
export function SourceOverviewStrip({
  buckets,
  trackId,
  width,
  duration,
  onZoom,
  range,
  recordingId,
  sourceRole,
  trackLabel,
  readPreview,
  previewRevision,
  pending,
  error,
}: SourceOverviewStripProps) {
  const pixelsPerMs = width / duration;
  const offsetPx = (range?.startTimelineMs ?? 0) * pixelsPerMs;
  const items = buckets
    .filter(
      (bucket) =>
        bucket.trackId === trackId &&
        (!range ||
          (bucket.startOffsetMs < range.endTimelineMs &&
            bucket.endOffsetMs > range.startTimelineMs)),
    )
    .map((bucket) => ({
      ...bucket,
      id: `${bucket.kind}:${bucket.startOffsetMs}`,
      startMs: bucket.startOffsetMs,
      endMs: bucket.endOffsetMs,
    }));
  const groups = groupTimelineEntries(items, pixelsPerMs);
  return (
    <div
      className="pointer-events-none absolute bottom-0 h-6"
      style={{
        left: offsetPx,
        width: range
          ? (range.endTimelineMs - range.startTimelineMs) * pixelsPerMs
          : '100%',
      }}
    >
      {groups.map((group) => (
        <TimelineEntryGroupButton
          key={`${previewRevision}:${group.id}:${group.startMs}:${group.endMs}`}
          group={group}
          count={group.items.reduce((count, bucket) => count + bucket.count, 0)}
          label={
            group.items[0]?.kind === 'speech'
              ? 'transcript entries'
              : group.items[0]?.kind === 'observation'
                ? 'screen changes'
                : 'activity events'
          }
          pixelsPerMs={pixelsPerMs}
          offsetPx={offsetPx}
          onZoom={onZoom}
          tooltipContent={
            sourceRole && recordingId && trackLabel ? (
              <TranscriptGroupPreview
                input={{
                  recordingId,
                  sourceRole,
                  startOffsetMs: group.startMs,
                  endOffsetMs: group.endMs,
                }}
                readPreview={readPreview}
                trackLabel={trackLabel}
                count={group.items.reduce(
                  (count, bucket) => count + bucket.count,
                  0,
                )}
                startMs={group.startMs}
                endMs={group.endMs}
              />
            ) : undefined
          }
        />
      ))}
      {pending || error ? (
        <span
          role="status"
          className="bg-background text-muted-foreground absolute bottom-1 left-1/2 -translate-x-1/2 px-1 text-xs whitespace-nowrap"
        >
          {error
            ? 'Details could not load. Refresh to retry.'
            : 'Loading details…'}
        </span>
      ) : null}
    </div>
  );
}
