import { useMemo } from 'react';
import { Progress } from '@/components/ui/progress';
import { formatDuration } from '../../../../library-format';
import {
  combinedProcessingStatus,
  groupTimelineProcessingRanges,
  type TimelineProcessingGroup,
} from './timeline-processing-groups';
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
const operations: readonly TimelineProcessingRange['operation'][] = [
  'transcription',
  'ocr',
  'screen-observations',
];
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
  const groups = useMemo(() => groupTimelineProcessingRanges(ranges), [ranges]);
  return (
    <div
      className="pointer-events-none absolute inset-x-0 top-6 bottom-13 z-30"
      data-timeline-processing
    >
      {groups
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
  range: TimelineProcessingGroup;
}
function ProcessingRangeBar({
  range,
  trackLabel,
  video,
  timelineWidthPx,
  totalDurationMs,
}: ProcessingRangeBarProps) {
  const descriptions = operations.flatMap((operation) => {
    const states = range.operations[operation];
    if (!states) return [];
    const status = combinedProcessingStatus(states);
    return [
      `${labels[operation]} · ${statuses[status]}${states.includes('failed') && status !== 'failed' ? ' · Needs attention' : ''}`,
    ];
  });
  const description = descriptions.join(' · ');
  const label = `${trackLabel} · ${description}`;
  return (
    <div
      className="absolute overflow-hidden"
      data-processing-range
      data-operation={Object.keys(range.operations).join(' ')}
      style={{
        left: (range.startOffsetMs / totalDurationMs) * timelineWidthPx,
        width: Math.max(
          2,
          ((range.endOffsetMs - range.startOffsetMs) / totalDurationMs) *
            timelineWidthPx,
        ),
        top: video ? '25%' : 0,
        height: video ? '50%' : '100%',
      }}
      title={`${label} · ${formatDuration(range.startOffsetMs)}–${formatDuration(range.endOffsetMs)}`}
    >
      <Progress
        className="timeline-processing-progress h-full rounded-sm border"
        data-processing-status={range.status}
        value={null}
        aria-label={label}
        aria-valuetext={description}
      />
      <span
        aria-hidden="true"
        className="bg-background/85 text-foreground absolute inset-x-1 top-1/2 -translate-y-1/2 truncate rounded-sm px-1 text-xs leading-4"
      >
        {description}
      </span>
    </div>
  );
}
