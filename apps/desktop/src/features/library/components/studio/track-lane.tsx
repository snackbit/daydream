import {
  useRef,
  type ButtonHTMLAttributes,
  type MouseEvent,
  type PointerEvent,
} from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryRecordingTrack,
  LibraryWaveformTiles,
  RecordingAiProcessingState,
} from '@daydream/shared';
import { cn } from '@/lib/utils';
import { buildDerivedLane } from './derived-ai-lane-strip';
import {
  studioClipId,
  studioColorById,
  type StudioClipDraft,
  type StudioLaneDraft,
  type StudioTimeRangeSelection,
} from './studio-timeline-store';
import type { StudioVisualFrame } from './studio-visual-observations';
import { TrackClip } from './track-clip';
import { TrackLaneHeader } from './track-lane-header';
import { TrackClipMedia } from './track-lane-media';
import { RedactionOverlay, TimeRangeOverlay } from './track-lane-overlays';
import {
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
  onSelectRedaction: (redactionId: string, additive: boolean) => void;
  onToggleTrackMuted: (trackId: string) => void;
  playbackAssets: LibraryPlaybackAssets | null;
  selectedClipIds: Set<string>;
  selectedLane: boolean;
  selectedRedactionIds: Set<string>;
  selectedVisualFrameIds: Set<string>;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
  timelineWidthPx: number;
  timeRanges: StudioTimeRangeSelection[];
  totalDurationMs: number;
  track: LibraryRecordingTrack;
  lane: StudioLaneDraft;
  visualFrames: StudioVisualFrame[];
  waveformTiles: LibraryWaveformTiles | null;
  onSelectVisualFrame: (visualFrameId: string, additive: boolean) => void;
}

export function TrackLane({
  aiProcessingState,
  clipDrafts,
  detail,
  dragHandleProps,
  mutedTrackIds,
  onSelectClip,
  onSelectLane,
  onSelectRedaction,
  onToggleTrackMuted,
  playbackAssets,
  selectedClipIds,
  selectedLane,
  selectedRedactionIds,
  selectedVisualFrameIds,
  thumbnailStatus,
  timelineWidthPx,
  timeRanges,
  totalDurationMs,
  track,
  lane,
  visualFrames,
  waveformTiles,
  onSelectVisualFrame,
}: TrackLaneProps) {
  const waveformTrack = waveformTiles?.tracks.find(
    (waveform) =>
      waveform.trackId === track.kind || waveform.kind === track.kind,
  );
  const audioTrack = track.kind !== 'video';
  const muted = mutedTrackIds.has(track.id);
  const redactions = detail.redactions.filter((redaction) =>
    redaction.trackIds.includes(track.id),
  );
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
      <TrackLaneHeader
        audioTrack={audioTrack}
        dragHandleProps={dragHandleProps}
        lane={lane}
        muted={muted}
        selectedLane={selectedLane}
        track={track}
        onSelectLane={onSelectLane}
        onToggleTrackMuted={onToggleTrackMuted}
      />
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
              onSelectClip={onSelectClip}
              derivedLane={derivedLane}
            >
              <TrackClipMedia
                segment={segment}
                thumbnailStatus={thumbnailStatus}
                thumbnails={
                  playbackAssets?.segments.find(
                    (playbackSegment) =>
                      playbackSegment.segmentId === segment.id,
                  )?.thumbnails ?? []
                }
                track={track}
                waveformTrack={waveformTrack}
              />
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
        {redactions.map((redaction) => (
          <RedactionOverlay
            key={redaction.id}
            redaction={redaction}
            selected={selectedRedactionIds.has(redaction.id)}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
            track={track}
            onSelectRedaction={onSelectRedaction}
          />
        ))}
        {track.kind === 'video' ? (
          <VisualFrameSegments
            selectedVisualFrameIds={selectedVisualFrameIds}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
            visualFrames={visualFrames}
            onSelectVisualFrame={onSelectVisualFrame}
          />
        ) : null}
      </div>
    </>
  );
}

function VisualFrameSegments({
  selectedVisualFrameIds,
  timelineWidthPx,
  totalDurationMs,
  visualFrames,
  onSelectVisualFrame,
}: {
  selectedVisualFrameIds: Set<string>;
  timelineWidthPx: number;
  totalDurationMs: number;
  visualFrames: StudioVisualFrame[];
  onSelectVisualFrame: (visualFrameId: string, additive: boolean) => void;
}) {
  if (visualFrames.length === 0) {
    return null;
  }

  return (
    <>
      {visualFrames.map((frame) => {
        const selected = selectedVisualFrameIds.has(frame.id);
        const startMs = Math.max(0, frame.frameTimestampMs);
        const endMs = Math.max(startMs + 1, frame.endTimestampMs);
        const left =
          (Math.min(startMs, totalDurationMs) / totalDurationMs) *
          timelineWidthPx;
        const width = Math.max(
          4,
          ((Math.min(endMs, totalDurationMs) -
            Math.min(startMs, totalDurationMs)) /
            totalDurationMs) *
            timelineWidthPx,
        );
        const title = visualFrameMarkerTitle(frame);

        return (
          <button
            aria-label={title}
            aria-pressed={selected}
            className={cn(
              'border-primary/80 bg-background/95 absolute bottom-1 z-20 h-6 overflow-hidden rounded-sm border px-1 text-[0.625rem] font-medium shadow-sm',
              'hover:bg-accent focus-visible:ring-ring focus-visible:ring-2 focus-visible:outline-none',
              selected && 'ring-ring ring-2',
            )}
            data-library-action
            key={frame.id}
            style={{
              left,
              width,
            }}
            title={title}
            type="button"
            onClick={(event) => {
              event.stopPropagation();
              onSelectVisualFrame(
                frame.id,
                event.shiftKey || event.metaKey || event.ctrlKey,
              );
            }}
          >
            <span className="block truncate">
              {frame.vlmObservations.length > 0 ? 'VLM' : 'OCR'}{' '}
              {frame.ocrRegions.length > 0 ? frame.ocrRegions.length : ''}
            </span>
          </button>
        );
      })}
    </>
  );
}

function visualFrameMarkerTitle(frame: StudioVisualFrame): string {
  const startSeconds = (frame.frameTimestampMs / 1_000).toFixed(1);
  const endSeconds = (frame.endTimestampMs / 1_000).toFixed(1);
  const vlmText =
    frame.vlmObservations.length > 0
      ? `${frame.vlmObservations.length} VLM`
      : 'no VLM';

  return `Visual frame ${startSeconds}s to ${endSeconds}s, ${vlmText}, ${frame.ocrRegions.length} OCR candidates`;
}
