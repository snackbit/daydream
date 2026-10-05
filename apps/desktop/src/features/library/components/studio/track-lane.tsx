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
import type { StudioInspectorSelection } from './studio-inspector-selection';
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
import { VisualFrameSegments } from './track-lane-visual-markers';
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
  onInspectTranscript: (selection: StudioInspectorSelection) => void;
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
  onSelectVisualFrame: (frame: StudioVisualFrame, additive: boolean) => void;
}

export function TrackLane({
  aiProcessingState,
  clipDrafts,
  detail,
  dragHandleProps,
  mutedTrackIds,
  onSelectClip,
  onInspectTranscript,
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
        trackId={track.id}
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
              onInspectTranscript={onInspectTranscript}
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
            trackLabel={track.label}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
            onSelectRedaction={onSelectRedaction}
          />
        ))}
        {track.kind === 'video' ? (
          <VisualFrameSegments
            visualFrameStates={aiProcessingState?.visualFrames ?? []}
            selectedVisualFrameIds={selectedVisualFrameIds}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
            visualFrames={visualFrames.filter(
              (frame) =>
                frame.sourceTrackId === undefined ||
                frame.sourceTrackId === track.id,
            )}
            onSelectVisualFrame={onSelectVisualFrame}
          />
        ) : null}
      </div>
    </>
  );
}
