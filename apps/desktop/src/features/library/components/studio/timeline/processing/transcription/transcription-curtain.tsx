import { Progress } from '@/components/ui/progress';
import { TRACK_TRANSCRIPTION_LANE_HEIGHT_PX } from '../../interactions/track-timeline-math';
import type { TranscriptionCurtain as CurtainModel } from './transcription-curtain-model';

export interface TranscriptionCurtainProps {
  curtain: CurtainModel;
  revealOffsetMs: number;
  trackLabel: string;
  timelineWidthPx: number;
  totalDurationMs: number;
  visibleRange: { startTimelineMs: number; endTimelineMs: number };
}
const statusLabels = {
  planned: 'Preparing',
  queued: 'Queued',
  running: 'Processing',
  paused: 'Paused',
  failed: 'Needs attention',
  cancelled: 'Cancelled',
};
export function TranscriptionCurtain({
  curtain,
  revealOffsetMs,
  trackLabel,
  timelineWidthPx,
  totalDurationMs,
  visibleRange,
}: TranscriptionCurtainProps) {
  const duration = curtain.endOffsetMs - curtain.startOffsetMs;
  const stops = curtain.ranges.flatMap((range) => [
    `transparent ${((range.startOffsetMs - curtain.startOffsetMs) / duration) * 100}%`,
    `black ${((range.startOffsetMs - curtain.startOffsetMs) / duration) * 100}%`,
    `black ${((range.endOffsetMs - curtain.startOffsetMs) / duration) * 100}%`,
    `transparent ${((range.endOffsetMs - curtain.startOffsetMs) / duration) * 100}%`,
  ]);
  const label = `${trackLabel} · Transcription · ${statusLabels[curtain.status]}`;
  const remainingStart = Math.max(
    curtain.startOffsetMs,
    Math.min(curtain.endOffsetMs, revealOffsetMs),
  );
  const labelStart = Math.max(remainingStart, visibleRange.startTimelineMs);
  return (
    <div
      data-timeline-transcription-curtain
      className="pointer-events-none absolute z-30"
      style={{
        left: (curtain.startOffsetMs / totalDurationMs) * timelineWidthPx,
        width: (duration / totalDurationMs) * timelineWidthPx,
        bottom: 4,
        height: TRACK_TRANSCRIPTION_LANE_HEIGHT_PX,
        maskImage: `linear-gradient(to right,${stops.join(',')})`,
      }}
    >
      <div
        className="timeline-transcription-curtain absolute inset-y-0 right-0 overflow-hidden rounded-sm border"
        data-processing-status={curtain.status}
        data-complete={curtain.progress === 1}
        style={{
          left:
            ((remainingStart - curtain.startOffsetMs) / duration) * 100 + '%',
        }}
      >
        <Progress
          aria-label={label}
          aria-valuetext={`${statusLabels[curtain.status]} · ${Math.round(curtain.progress * 100)}%`}
          className="sr-only"
          value={curtain.progress > 0 ? curtain.progress * 100 : null}
        />
        <span
          aria-hidden="true"
          className="bg-background/90 text-foreground absolute top-1/2 -translate-y-1/2 truncate rounded-sm px-1 text-xs leading-4"
          style={{
            left: Math.max(
              0,
              ((labelStart - remainingStart) / totalDurationMs) *
                timelineWidthPx +
                4,
            ),
            maxWidth: Math.max(
              0,
              ((Math.min(curtain.endOffsetMs, visibleRange.endTimelineMs) -
                labelStart) /
                totalDurationMs) *
                timelineWidthPx -
                8,
            ),
          }}
        >
          Transcription · {statusLabels[curtain.status]}
          {curtain.progress > 0
            ? ` · ${Math.round(curtain.progress * 100)}%`
            : ''}
        </span>
      </div>
    </div>
  );
}
