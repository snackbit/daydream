import {
  useRef,
  type ButtonHTMLAttributes,
  type MouseEvent,
  type PointerEvent,
  type ReactNode,
} from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryRecordingSegment,
  LibraryRecordingTrack,
  LibraryWaveformTileTrack,
  LibraryWaveformTiles,
  RecordingAiProcessingState,
} from '@daydream/shared';
import { GripVertical, Volume2, VolumeX } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { buildDerivedLane, DerivedAiLaneStrip } from './derived-ai-lane-strip';
import {
  studioClipId,
  studioColorById,
  type StudioClipDraft,
  type StudioLaneDraft,
  type StudioTimeRangeSelection,
} from './studio-timeline-store';
import {
  TRACK_DERIVED_LANE_HEIGHT_PX,
  TRACK_LANE_HEIGHT_PX,
  TRACK_SELECTION_DRAG_THRESHOLD_PX,
} from './track-timeline-math';

export interface TrackLaneProps {
  aiProcessingState: RecordingAiProcessingState | null;
  clipDrafts: Record<string, StudioClipDraft>;
  detail: LibraryRecordingDetail;
  dragHandleProps: ButtonHTMLAttributes<HTMLButtonElement>;
  mutedTrackIds: Set<string>;
  onSelectClip: (clipId: string, additive: boolean) => void;
  onSelectLane: (trackId: string, additive: boolean) => void;
  onToggleTrackMuted: (trackId: string) => void;
  playbackAssets: LibraryPlaybackAssets | null;
  selectedClipIds: Set<string>;
  selectedLane: boolean;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
  timelineWidthPx: number;
  timeRanges: StudioTimeRangeSelection[];
  totalDurationMs: number;
  track: LibraryRecordingTrack;
  lane: StudioLaneDraft;
  waveformTiles: LibraryWaveformTiles | null;
}

export function TrackLane({
  aiProcessingState,
  clipDrafts,
  detail,
  dragHandleProps,
  mutedTrackIds,
  onSelectClip,
  onSelectLane,
  onToggleTrackMuted,
  playbackAssets,
  selectedClipIds,
  selectedLane,
  thumbnailStatus,
  timelineWidthPx,
  timeRanges,
  totalDurationMs,
  track,
  lane,
  waveformTiles,
}: TrackLaneProps) {
  const waveformTrack = waveformTiles?.tracks.find(
    (waveform) =>
      waveform.trackId === track.kind || waveform.kind === track.kind,
  );
  const audioTrack = track.kind !== 'video';
  const muted = mutedTrackIds.has(track.id);
  const color = studioColorById(lane.colorId);
  const derivedLane = buildDerivedLane({
    aiProcessingState,
    track,
  });
  const laneContentClickRef = useRef<{
    moved: boolean;
    x: number;
    y: number;
  } | null>(null);

  function handleLaneContentPointerDown(event: PointerEvent<HTMLDivElement>) {
    if (
      event.target instanceof Element &&
      event.target.closest('[data-library-action]')
    ) {
      laneContentClickRef.current = null;
      return;
    }

    laneContentClickRef.current = {
      moved: false,
      x: event.clientX,
      y: event.clientY,
    };
  }

  function handleLaneContentPointerMove(event: PointerEvent<HTMLDivElement>) {
    const clickState = laneContentClickRef.current;
    if (!clickState) {
      return;
    }

    if (
      Math.hypot(event.clientX - clickState.x, event.clientY - clickState.y) >
      TRACK_SELECTION_DRAG_THRESHOLD_PX
    ) {
      clickState.moved = true;
    }
  }

  function handleLaneContentClick(event: MouseEvent<HTMLDivElement>) {
    if (
      event.target instanceof Element &&
      event.target.closest('[data-library-action]')
    ) {
      return;
    }

    const clickState = laneContentClickRef.current;
    laneContentClickRef.current = null;
    if (clickState?.moved) {
      return;
    }

    onSelectLane(track.id, event.shiftKey || event.metaKey || event.ctrlKey);
  }

  return (
    <>
      <div
        className={cn(
          'bg-background group-hover/lane:bg-muted/70 sticky left-0 z-10 flex min-w-0 flex-col border-r border-b transition-colors',
          selectedLane && 'bg-accent text-accent-foreground',
        )}
        data-track-id={track.id}
        style={{ height: TRACK_LANE_HEIGHT_PX }}
      >
        <div className="flex min-h-0 flex-1 items-center gap-1 p-2">
          <Button
            aria-label={`Reorder ${lane.label}`}
            className="cursor-grab active:cursor-grabbing"
            data-library-action
            size="icon-xs"
            type="button"
            variant="ghost"
            {...dragHandleProps}
          >
            <GripVertical className="size-3" />
          </Button>
          {audioTrack ? (
            <Button
              aria-label={muted ? `Unmute ${lane.label}` : `Mute ${lane.label}`}
              data-library-action
              size="icon"
              variant={muted ? 'outline' : 'ghost'}
              onClick={() => onToggleTrackMuted(track.id)}
            >
              {muted ? (
                <VolumeX className="size-3.5" />
              ) : (
                <Volume2 className="size-3.5" />
              )}
            </Button>
          ) : null}
          <button
            aria-label={`Select ${lane.label}`}
            aria-pressed={selectedLane}
            className="hover:bg-muted focus-visible:ring-ring/30 min-w-0 flex-1 rounded-sm px-1 py-0.5 text-left transition-colors outline-none focus-visible:ring-2"
            data-library-action
            type="button"
            onClick={(event) =>
              onSelectLane(
                track.id,
                event.shiftKey || event.metaKey || event.ctrlKey,
              )
            }
          >
            <span className="flex items-center gap-2">
              <span
                aria-hidden="true"
                className="size-2.5 shrink-0 rounded-full"
                style={{ backgroundColor: color.strong }}
              />
              <span className="min-w-0">
                <span className="block truncate text-xs font-medium">
                  {lane.label}
                </span>
                <span className="text-muted-foreground block truncate text-xs">
                  {lane.description}
                </span>
              </span>
            </span>
          </button>
        </div>
        <div
          className="text-muted-foreground flex h-6 items-center gap-2 border-t px-2 text-[0.625rem]"
          title={derivedLane.description}
        >
          <span
            aria-hidden="true"
            className="size-1.5 shrink-0 rounded-full"
            style={{ backgroundColor: color.strong }}
          />
          <span className="truncate">{derivedLane.label}</span>
        </div>
      </div>
      <div
        className={cn(
          'bg-muted/60 group-hover/lane:bg-muted/80 relative cursor-pointer overflow-hidden border-b transition-colors',
          selectedLane && 'bg-accent/20',
        )}
        data-lane-content
        data-track-id={track.id}
        style={{ height: TRACK_LANE_HEIGHT_PX, width: timelineWidthPx }}
        onClick={handleLaneContentClick}
        onPointerDown={handleLaneContentPointerDown}
        onPointerMove={handleLaneContentPointerMove}
      >
        {detail.segments.map((segment) => {
          const clipId = studioClipId(track.id, segment.id);
          const clip = clipDrafts[clipId];

          return (
            <TrackClip
              clipId={clipId}
              color={color}
              key={clipId}
              segment={segment}
              selected={selectedClipIds.has(clipId)}
              title={clip?.title ?? `Segment ${segment.segmentIndex}`}
              timelineWidthPx={timelineWidthPx}
              totalDurationMs={totalDurationMs}
              track={track}
              waveformTrack={waveformTrack}
              onSelectClip={onSelectClip}
            >
              {track.kind === 'video' ? (
                <VideoThumbnailStrip
                  isLoading={thumbnailStatus === 'loading'}
                  thumbnails={
                    playbackAssets?.segments.find(
                      (playbackSegment) =>
                        playbackSegment.segmentId === segment.id,
                    )?.thumbnails ?? []
                  }
                />
              ) : waveformTrack ? (
                <WaveformTileLayer
                  segment={segment}
                  waveformTrack={waveformTrack}
                />
              ) : audioTrack ? (
                <WaveformSilenceLine />
              ) : null}
            </TrackClip>
          );
        })}
        {timeRanges.map((range) => (
          <TimeRangeOverlay
            key={range.id}
            range={range}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
          />
        ))}
        <DerivedAiLaneStrip
          color={color.strong}
          lane={derivedLane}
          timelineWidthPx={timelineWidthPx}
          totalDurationMs={totalDurationMs}
        />
      </div>
    </>
  );
}

function TrackClip({
  children,
  clipId,
  color,
  onSelectClip,
  segment,
  selected,
  timelineWidthPx,
  title,
  totalDurationMs,
  track,
  waveformTrack,
}: {
  children: ReactNode;
  clipId: string;
  color: ReturnType<typeof studioColorById>;
  onSelectClip: (clipId: string, additive: boolean) => void;
  segment: LibraryRecordingSegment;
  selected: boolean;
  timelineWidthPx: number;
  title: string;
  totalDurationMs: number;
  track: LibraryRecordingTrack;
  waveformTrack: LibraryWaveformTileTrack | undefined;
}) {
  const finalized = segment.fileExists && segment.state === 'finalized';

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
      data-library-action
      style={{
        backgroundColor: finalized ? color.fill : 'hsl(var(--muted))',
        borderColor: selected ? color.strong : color.border,
        left: (segment.playbackOffsetMs / totalDurationMs) * timelineWidthPx,
        bottom: TRACK_DERIVED_LANE_HEIGHT_PX + 4,
        width: Math.max(
          4,
          ((segment.durationMs ?? 0) / totalDurationMs) * timelineWidthPx,
        ),
      }}
      title={`${track.label} ${title}`}
      type="button"
      onClick={(event) =>
        onSelectClip(clipId, event.shiftKey || event.metaKey || event.ctrlKey)
      }
    >
      <div
        className="absolute inset-x-0 top-0 z-10 flex h-4 items-center truncate px-1 text-[0.625rem] font-medium text-white"
        style={{ backgroundColor: color.strong }}
      >
        <span className="truncate">{title}</span>
      </div>
      <div className="bg-background/60 absolute inset-x-0 top-4 bottom-0 overflow-hidden">
        {children}
        {!waveformTrack && track.kind !== 'video' ? (
          <WaveformSilenceLine />
        ) : null}
      </div>
    </button>
  );
}

function TimeRangeOverlay({
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

function WaveformTileLayer({
  segment,
  waveformTrack,
}: {
  segment: LibraryRecordingSegment;
  waveformTrack: LibraryWaveformTileTrack;
}) {
  const segmentDurationMs = Math.max(1, segment.durationMs ?? 0);
  const tiles = waveformTrack.tiles.filter(
    (tile) => tile.segmentId === segment.id,
  );
  if (tiles.length === 0) {
    return <WaveformSilenceLine />;
  }

  return (
    <div className="absolute inset-0">
      <WaveformSilenceLine />
      {tiles.map((tile) => {
        const tileStartInSegmentMs =
          tile.timelineStartMs - segment.playbackOffsetMs;
        const clippedStartMs = Math.max(0, tileStartInSegmentMs);
        const clippedEndMs = Math.min(
          segmentDurationMs,
          tileStartInSegmentMs + tile.durationMs,
        );
        if (clippedEndMs <= clippedStartMs) {
          return null;
        }

        return (
          <div
            className="absolute inset-y-0"
            key={`${tile.segmentId}-${tile.streamIndex}-${tile.binMs}-${tile.timelineStartMs}`}
            style={{
              left: `${(clippedStartMs / segmentDurationMs) * 100}%`,
              width: `${
                ((clippedEndMs - clippedStartMs) / segmentDurationMs) * 100
              }%`,
            }}
          >
            <WaveformShape samples={tile.peakSamples} />
          </div>
        );
      })}
    </div>
  );
}

function WaveformSilenceLine() {
  return (
    <div
      aria-label="Audio silence baseline"
      className="bg-primary/30 absolute inset-x-0 top-1/2 h-px -translate-y-1/2"
    />
  );
}

function VideoThumbnailStrip({
  isLoading,
  thumbnails,
}: {
  isLoading: boolean;
  thumbnails: Array<{ offsetMs: number; url: string }>;
}) {
  if (thumbnails.length === 0) {
    return (
      <div className="text-muted-foreground flex h-full items-center justify-center px-2 text-[0.625rem]">
        {isLoading ? 'Thumbnails loading...' : 'Thumbnails unavailable'}
      </div>
    );
  }

  return (
    <div className="flex h-full w-full">
      {thumbnails.map((thumbnail) => (
        <img
          alt=""
          aria-label="Video timeline thumbnail"
          className="h-full min-w-0 flex-1 border-r object-cover last:border-r-0"
          draggable={false}
          key={`${thumbnail.url}-${thumbnail.offsetMs}`}
          src={thumbnail.url}
        />
      ))}
    </div>
  );
}

function WaveformShape({ samples }: { samples: number[] }) {
  if (samples.length === 0) {
    return null;
  }
  const points = samples.map((sample, index) => {
    const x = samples.length === 1 ? 0 : (index / (samples.length - 1)) * 100;
    const y = 50 - Math.max(0, Math.min(1, sample)) * 48;

    return `${x.toFixed(3)},${y.toFixed(3)}`;
  });
  const reflected = [...samples].reverse().map((sample, index) => {
    const x =
      samples.length === 1
        ? 0
        : ((samples.length - 1 - index) / (samples.length - 1)) * 100;
    const y = 50 + Math.max(0, Math.min(1, sample)) * 48;

    return `${x.toFixed(3)},${y.toFixed(3)}`;
  });

  return (
    <svg
      aria-label="Audio waveform"
      className="h-full w-full"
      preserveAspectRatio="none"
      viewBox="0 0 100 100"
    >
      <polygon
        fill="currentColor"
        opacity="0.72"
        points={`${points.join(' ')} ${reflected.join(' ')}`}
      />
    </svg>
  );
}
