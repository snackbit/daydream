import { formatDuration } from '../../../../library-format';
import { TRACK_LABEL_WIDTH_PX } from '../interactions/track-timeline-math';
import type { StudioTimeRangeSelection } from '../state/studio-timeline-store';

export interface TimelineRangeLabelsProps {
  ranges: StudioTimeRangeSelection[];
  timelineWidthPx: number;
  totalDurationMs: number;
}

export function TimelineRangeLabels({
  ranges,
  timelineWidthPx,
  totalDurationMs,
}: TimelineRangeLabelsProps) {
  return (
    <>
      {ranges.map((range) => {
        const left = (range.startMs / totalDurationMs) * timelineWidthPx;
        const width =
          ((range.endMs - range.startMs) / totalDurationMs) * timelineWidthPx;

        return (
          <div
            aria-label="Selected time range"
            title={`${formatDuration(range.startMs)} – ${formatDuration(range.endMs)}`}
            className="bg-destructive/80 text-destructive-foreground pointer-events-none absolute top-0 z-40 overflow-hidden rounded-b-sm px-1 py-px text-xs font-medium whitespace-nowrap"
            key={range.id}
            style={{
              left: TRACK_LABEL_WIDTH_PX + left,
              width: Math.max(1, width),
            }}
          >
            {formatDuration(range.startMs)} - {formatDuration(range.endMs)}
          </div>
        );
      })}
    </>
  );
}
