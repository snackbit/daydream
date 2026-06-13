import type { LibraryRecordingRedaction } from '@daydream/shared';
import { cn } from '@/lib/utils';
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
      className="pointer-events-none absolute inset-y-1 z-20 rounded-sm border border-red-500 bg-red-500/12"
      style={{ left, width }}
    />
  );
}

export function RedactionOverlay({
  onSelectRedaction,
  redaction,
  selected,
  trackLabel,
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
    <button
      aria-label={`${trackLabel} redacted range`}
      aria-pressed={selected}
      className={cn(
        'absolute inset-y-1 z-30 overflow-hidden rounded-sm border border-neutral-900 bg-neutral-700 text-left text-[0.625rem] font-medium text-white shadow-inner outline-none',
        'focus-visible:ring-ring/40 focus-visible:ring-2',
        selected && 'ring-ring ring-offset-background ring-2 ring-offset-1',
      )}
      data-library-action
      style={{
        left,
        width: Math.max(4, width),
      }}
      title={`${trackLabel} redacted`}
      type="button"
      onClick={(event) => {
        event.stopPropagation();
        onSelectRedaction(
          redaction.id,
          event.shiftKey || event.metaKey || event.ctrlKey,
        );
      }}
    >
      <span className="absolute inset-x-0 top-0 flex h-4 items-center bg-neutral-950 px-1 text-white">
        <span className="truncate">Redacted</span>
      </span>
      <span className="sr-only">
        {redaction.startOffsetMs} to {redaction.endOffsetMs} ms
      </span>
    </button>
  );
}
