import type { CSSProperties } from 'react';
import { Progress } from '@/components/ui/progress';
import { formatDuration } from '../../../../library-format';
import type {
  TimelineProcessingRange,
  TimelineProcessingStatus,
} from './timeline-processing-ranges';

export interface TimelineProcessingOverlayProps {
  ranges: readonly TimelineProcessingRange[];
  trackLabel: string;
  video: boolean;
  timelineWidthPx: number;
  totalDurationMs: number;
  visibleRange: { startTimelineMs: number; endTimelineMs: number };
}
const labels = {
  transcription: 'Transcription',
  ocr: 'OCR',
  'screen-observations': 'Screen observations',
};
const statuses: Record<TimelineProcessingStatus, string> = {
  planned: 'Preparing',
  queued: 'Queued',
  running: 'Processing',
  paused: 'Paused',
  failed: 'Needs attention',
  cancelled: 'Cancelled',
};

export function TimelineProcessingOverlay({
  ranges,
  trackLabel,
  video,
  timelineWidthPx,
  totalDurationMs,
  visibleRange,
}: TimelineProcessingOverlayProps) {
  return (
    <div
      className="pointer-events-none absolute inset-x-0 top-6 bottom-13 z-30"
      data-timeline-processing
    >
      {ranges
        .filter(
          (range) =>
            range.startOffsetMs < visibleRange.endTimelineMs &&
            range.endOffsetMs > visibleRange.startTimelineMs,
        )
        .map((range) => (
          <ProcessingRangeBar
            key={range.id}
            range={range}
            trackLabel={trackLabel}
            video={video}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
          />
        ))}
    </div>
  );
}

interface ProcessingRangeBarProps extends Pick<
  TimelineProcessingOverlayProps,
  'trackLabel' | 'video' | 'timelineWidthPx' | 'totalDurationMs'
> {
  range: TimelineProcessingRange;
}
function ProcessingRangeBar({
  range,
  trackLabel,
  video,
  timelineWidthPx,
  totalDurationMs,
}: ProcessingRangeBarProps) {
  const label = `${trackLabel} · ${labels[range.operation]} · ${statuses[range.status]}`;
  const measuring = range.progress > 0;
  const percentage = Math.round(range.progress * 100);
  const progressStyle: CSSProperties & {
    '--timeline-processing-complete': string;
  } = { '--timeline-processing-complete': `${percentage}%` };
  return (
    <div
      className="absolute overflow-hidden"
      data-processing-range
      data-operation={range.operation}
      style={{
        left: (range.startOffsetMs / totalDurationMs) * timelineWidthPx,
        width: Math.max(
          2,
          ((range.endOffsetMs - range.startOffsetMs) / totalDurationMs) *
            timelineWidthPx,
        ),
        top: video && range.operation === 'screen-observations' ? '50%' : 0,
        height: video ? '50%' : '100%',
      }}
      title={`${label} · ${formatDuration(range.startOffsetMs)}–${formatDuration(range.endOffsetMs)}`}
    >
      <Progress
        className="timeline-processing-progress h-full rounded-sm border"
        data-processing-status={range.status}
        value={measuring ? percentage : null}
        aria-label={label}
        aria-valuetext={
          measuring
            ? `${statuses[range.status]} · ${percentage}%`
            : statuses[range.status]
        }
        style={progressStyle}
      />
      <span
        aria-hidden="true"
        className="bg-background/85 text-foreground absolute inset-x-1 top-1/2 -translate-y-1/2 truncate rounded-sm px-1 text-xs leading-4"
      >
        {labels[range.operation]} · {statuses[range.status]}
        {measuring ? ` · ${percentage}%` : ''}
      </span>
    </div>
  );
}
