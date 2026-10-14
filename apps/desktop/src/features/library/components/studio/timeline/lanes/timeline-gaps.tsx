import type { LibraryRecordingSegment } from '@daydream/shared';
import { formatTimestamp } from '../../../../library-format';
import {
  TRACK_LABEL_WIDTH_PX,
  TRACK_RULER_HEIGHT_PX,
} from '../interactions/track-timeline-math';

export interface TimelineGapsProps {
  segments: LibraryRecordingSegment[];
  timelineWidthPx: number;
  totalDurationMs: number;
}
export function TimelineGaps({
  segments,
  timelineWidthPx,
  totalDurationMs,
}: TimelineGapsProps) {
  return segments
    .filter((s) => s.pauseAfterMs > 0)
    .map((segment) => {
      const start = segment.playbackOffsetMs + (segment.durationMs ?? 0);
      const clock = segment.startedAtMs + (segment.durationMs ?? 0);
      return (
        <div
          key={segment.id}
          role="img"
          aria-label={`Not recorded, ${formatTimestamp(clock)}–${formatTimestamp(clock + segment.pauseAfterMs)}`}
          title={`Not recorded · ${formatTimestamp(clock)}–${formatTimestamp(clock + segment.pauseAfterMs)}`}
          className="studio-redaction-pattern bg-muted/90 border-muted-foreground/40 pointer-events-none absolute bottom-0 z-20 overflow-hidden border-x"
          style={{
            top: TRACK_RULER_HEIGHT_PX,
            left:
              TRACK_LABEL_WIDTH_PX +
              (start / totalDurationMs) * timelineWidthPx,
            width: (segment.pauseAfterMs / totalDurationMs) * timelineWidthPx,
          }}
        >
          <span className="text-muted-foreground px-2 text-xs whitespace-nowrap">
            Not recorded
          </span>
        </div>
      );
    });
}
