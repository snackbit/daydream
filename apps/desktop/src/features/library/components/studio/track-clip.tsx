import type { ReactNode } from 'react';
import type {
  LibraryRecordingSegment,
  LibraryRecordingTrack,
} from '@daydream/shared';
import { cn } from '@/lib/utils';
import {
  SegmentTranscriptionLayer,
  type DerivedAiLane,
} from './derived-ai-lane-strip';
import type { studioColorById } from './studio-timeline-store';
import { TRACK_DERIVED_LANE_HEIGHT_PX } from './track-timeline-math';

export function TrackClip({
  children,
  clipId,
  color,
  derivedLane,
  onSelectClip,
  segment,
  selected,
  timelineWidthPx,
  title,
  totalDurationMs,
  track,
}: {
  children: ReactNode;
  clipId: string;
  color: ReturnType<typeof studioColorById>;
  derivedLane: DerivedAiLane;
  onSelectClip: (clipId: string, additive: boolean) => void;
  segment: LibraryRecordingSegment;
  selected: boolean;
  timelineWidthPx: number;
  title: string;
  totalDurationMs: number;
  track: LibraryRecordingTrack;
}) {
  const finalized = segment.fileExists && segment.state === 'finalized';
  const hasTranscriptionLayer = track.kind !== 'video';

  return (
    <button
      aria-label={`${track.label} ${title}`}
      aria-pressed={selected}
      className={cn(
        'absolute top-1 overflow-hidden rounded-sm border text-left shadow-xs transition-[border-color,box-shadow,transform] outline-none',
        'focus-visible:ring-ring/40 focus-visible:ring-2',
        selected && 'ring-ring ring-offset-background ring-2 ring-offset-1',
        !finalized && 'opacity-60',
      )}
      style={{
        backgroundColor: finalized ? color.fill : 'hsl(var(--muted))',
        borderColor: selected ? color.strong : color.border,
        left: (segment.playbackOffsetMs / totalDurationMs) * timelineWidthPx,
        bottom: 4,
        width: Math.max(
          4,
          ((segment.durationMs ?? 0) / totalDurationMs) * timelineWidthPx,
        ),
      }}
      title={`${track.label} ${title}`}
      type="button"
      onClick={(event) => {
        event.stopPropagation();
        onSelectClip(clipId, event.shiftKey || event.metaKey || event.ctrlKey);
      }}
    >
      <div
        className="absolute inset-x-0 top-0 z-10 flex h-4 items-center truncate px-1 text-[0.625rem] font-medium text-white"
        data-library-action
        style={{ backgroundColor: color.strong }}
      >
        <span className="truncate">{title}</span>
      </div>
      <div
        className="bg-background/60 absolute inset-x-0 top-4 overflow-hidden"
        style={{
          bottom: hasTranscriptionLayer ? TRACK_DERIVED_LANE_HEIGHT_PX : 0,
        }}
      >
        {children}
      </div>
      {hasTranscriptionLayer ? (
        <SegmentTranscriptionLayer
          color={color.strong}
          lane={derivedLane}
          segment={segment}
        />
      ) : null}
    </button>
  );
}
