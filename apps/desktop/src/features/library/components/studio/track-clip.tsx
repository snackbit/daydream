import type { MouseEvent, ReactNode } from 'react';
import type {
  LibraryRecordingSegment,
  LibraryRecordingTrack,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import {
  SegmentTranscriptionLayer,
  type DerivedAiLane,
} from './derived-ai-lane-strip';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import type { studioColorById } from './studio-timeline-store';
import { TRACK_TRANSCRIPTION_LANE_HEIGHT_PX } from './track-timeline-math';

export function TrackClip({
  children,
  clipId,
  color,
  derivedLane,
  onSelectClip,
  onInspectTranscript,
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
  onInspectTranscript: (selection: StudioInspectorSelection) => void;
  segment: LibraryRecordingSegment;
  selected: boolean;
  timelineWidthPx: number;
  title: string;
  totalDurationMs: number;
  track: LibraryRecordingTrack;
}) {
  const finalized = segment.fileExists && segment.state === 'finalized';
  const hasTranscriptionLayer = track.kind !== 'video';

  function handleSelectClip(event: MouseEvent<HTMLButtonElement>) {
    event.stopPropagation();
    onSelectClip(clipId, event.shiftKey || event.metaKey || event.ctrlKey);
  }

  return (
    <div
      className="absolute top-1 overflow-hidden rounded-sm border transition-[border-color,box-shadow,transform]"
      data-clip-id={clipId}
      data-selected={selected}
      style={{
        backgroundColor: finalized
          ? `color-mix(in oklch, ${color.fill} 12%, var(--card))`
          : 'var(--muted)',
        borderColor: selected
          ? 'var(--foreground)'
          : `color-mix(in oklch, ${color.border} 24%, var(--border))`,
        left: (segment.playbackOffsetMs / totalDurationMs) * timelineWidthPx,
        bottom: track.kind === 'video' ? 52 : 4,
        width: Math.max(
          4,
          ((segment.durationMs ?? 0) / totalDurationMs) * timelineWidthPx,
        ),
      }}
    >
      <Button
        aria-label={`${track.label} ${title}`}
        aria-pressed={selected}
        className={cn(
          'absolute inset-0 z-0 flex h-full w-full flex-col items-stretch justify-start overflow-hidden rounded-sm px-0 py-0 text-left font-normal whitespace-normal outline-none',
          'focus-visible:ring-ring focus-visible:ring-2',
          selected && 'ring-primary ring-1 ring-inset',
          !finalized && 'opacity-60',
        )}
        title={`${track.label} ${title}`}
        type="button"
        variant="ghost"
        onClick={handleSelectClip}
      >
        <div
          className="bg-muted/80 text-foreground absolute inset-x-0 top-0 z-10 flex h-5 items-center truncate border-b px-1 text-xs font-medium"
          data-library-action
        >
          <span className="truncate">{title}</span>
        </div>
        <div
          className="bg-background/60 absolute inset-x-0 top-4 overflow-hidden"
          style={{
            bottom: hasTranscriptionLayer
              ? TRACK_TRANSCRIPTION_LANE_HEIGHT_PX
              : 0,
          }}
        >
          {children}
        </div>
      </Button>
      {hasTranscriptionLayer ? (
        <SegmentTranscriptionLayer
          color={`color-mix(in oklch, ${color.strong} 44%, var(--muted-foreground))`}
          lane={derivedLane}
          segment={segment}
          track={track}
          onInspectTranscript={onInspectTranscript}
        />
      ) : null}
    </div>
  );
}
