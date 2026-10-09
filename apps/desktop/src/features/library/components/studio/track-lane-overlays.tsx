import type { LibraryRecordingRedaction } from '@daydream/shared';
import type { StudioTimeRangeSelection } from './studio-timeline-store';

export function TimeRangeOverlay({
  range,
  timelineWidthPx,
  totalDurationMs,
}: {
  range: StudioTimeRangeSelection;
  timelineWidthPx: number;
  totalDurationMs: number;
}) {
  const left = (range.startMs / totalDurationMs) * timelineWidthPx;
  const width =
    ((range.endMs - range.startMs) / totalDurationMs) * timelineWidthPx;

  return (
    <div
      aria-hidden="true"
      className="border-primary/60 bg-primary/10 pointer-events-none absolute inset-y-1 z-20 rounded-sm border"
      style={{ left, width }}
    />
  );
}

export function RedactionOverlay({
  redaction,
  timelineWidthPx,
  totalDurationMs,
}: {
  onSelectRedaction: (redactionId: string, additive: boolean) => void;
  redaction: LibraryRecordingRedaction;
  selected: boolean;
  trackLabel: string;
  timelineWidthPx: number;
  totalDurationMs: number;
}) {
  const left = (redaction.startOffsetMs / totalDurationMs) * timelineWidthPx;
  const width =
    ((redaction.endOffsetMs - redaction.startOffsetMs) / totalDurationMs) *
    timelineWidthPx;

  return (
    <div
      aria-hidden="true"
      className="studio-redaction-pattern bg-muted border-muted-foreground/40 border-y-border pointer-events-none absolute inset-y-0 z-30 border-x border-y"
      style={{
        left,
        width: Math.max(4, width),
      }}
    />
  );
}
