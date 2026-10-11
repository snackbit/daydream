import {
  useRef,
  useMemo,
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
  ReprocessingTarget,
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
import { TimelineProcessingOverlay } from './timeline/processing/timeline-processing-overlay';
import {
  timelineProcessingRanges,
  type TimelinePlannedScope,
} from './timeline/processing/timeline-processing-ranges';
import { TranscriptionCurtain } from './timeline/processing/transcription-curtain';
import { transcriptionCurtain } from './timeline/processing/transcription-curtain-model';
import { useTranscriptionCurtainMotion } from './timeline/processing/use-transcription-curtain-motion';
import { TrackClip } from './track-clip';
import { TrackLaneHeader } from './track-lane-header';
import { TrackClipMedia } from './track-lane-media';
import { RedactionOverlay, TimeRangeOverlay } from './track-lane-overlays';
import { VisualFrameSegments } from './track-lane-visual-markers';
import {
  trackLaneHeightPxForKind,
  TRACK_SELECTION_DRAG_THRESHOLD_PX,
} from './track-timeline-math';

export interface TrackLaneProps {
  plannedProcessingScopes?: readonly TimelinePlannedScope[] | undefined;
  onReprocess?: ((targets: ReprocessingTarget[]) => void) | undefined;
  visibleRange: { startTimelineMs: number; endTimelineMs: number };
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
  plannedProcessingScopes,
  visibleRange,
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
  onReprocess,
}: TrackLaneProps) {
  const curtain = useMemo(
    () =>
      transcriptionCurtain({
        detail,
        track,
        state: aiProcessingState,
        ...(plannedProcessingScopes
          ? { plannedScopes: plannedProcessingScopes }
          : {}),
      }),
    [detail, track, aiProcessingState, plannedProcessingScopes],
  );
  const curtainMotion = useTranscriptionCurtainMotion(
    curtain,
    `${detail.id}:${track.id}`,
  );
  const processingRanges = useMemo(
    () =>
      timelineProcessingRanges({
        detail,
        track,
        state: aiProcessingState,
        ...(plannedProcessingScopes
          ? { plannedScopes: plannedProcessingScopes }
          : {}),
      }),
    [detail, track, aiProcessingState, plannedProcessingScopes],
  );
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
  const derivedLane = useMemo(
    () => buildDerivedLane({ aiProcessingState, track }),
    [aiProcessingState, track],
  );
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
        heightPx={trackLaneHeightPxForKind(track.kind)}
        dragHandleProps={dragHandleProps}
        lane={lane}
        muted={muted}
        selectedLane={selectedLane}
        trackId={track.id}
        onSelectLane={onSelectLane}
        onToggleTrackMuted={onToggleTrackMuted}
        onReprocess={onReprocess}
      />
      <div
        className={cn(
          'bg-muted/60 group-hover/lane:bg-muted/80 relative isolate z-0 cursor-pointer overflow-hidden border-b transition-colors',
          selectedLane && 'bg-accent/20',
        )}
        data-lane-content
        data-track-id={track.id}
        style={{
          height: trackLaneHeightPxForKind(track.kind),
          width: timelineWidthPx,
        }}
        onClick={handleLaneContentClick}
        onPointerDown={handleLaneContentPointerDown}
        onPointerMove={handleLaneContentPointerMove}
      >
        {detail.segments
          .filter(
            (segment) =>
              (track.kind !== 'video' ||
                !segment.videoStreams?.length ||
                segment.videoStreams.some(
                  (source) => source.trackId === track.id,
                )) &&
              segment.playbackOffsetMs < visibleRange.endTimelineMs &&
              segment.playbackOffsetMs + (segment.durationMs ?? 1) >
                visibleRange.startTimelineMs,
          )
          .map((segment) => {
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
                hiddenTranscriptRanges={curtain?.pendingRanges ?? []}
                publishedTranscriptRanges={curtain?.publishedRanges ?? []}
                transcriptRevealOffsetMs={
                  curtainMotion.visible ? curtainMotion.revealOffsetMs : null
                }
              >
                <TrackClipMedia
                  segment={segment}
                  thumbnailStatus={thumbnailStatus}
                  thumbnails={
                    playbackAssets?.segments
                      .find(
                        (playbackSegment) =>
                          playbackSegment.segmentId === segment.id,
                      )
                      ?.videos?.find((video) => video.trackId === track.id)
                      ?.thumbnails ??
                    playbackAssets?.segments.find(
                      (segmentAsset) => segmentAsset.segmentId === segment.id,
                    )?.thumbnails ??
                    []
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
            visualFrameStates={(aiProcessingState?.visualFrames ?? []).filter(
              (frame) =>
                (frame.sourceFrameRef.match(/:stream:([^:]+):/)?.[1] ??
                  detail.tracks.find((candidate) => candidate.kind === 'video')
                    ?.id) === track.id &&
                frame.frameTimestampMs >= visibleRange.startTimelineMs &&
                frame.frameTimestampMs < visibleRange.endTimelineMs,
            )}
            selectedVisualFrameIds={selectedVisualFrameIds}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
            visualFrames={visualFrames
              .filter(
                (frame) =>
                  frame.sourceTrackId === undefined ||
                  frame.sourceTrackId === track.id,
              )
              .filter(
                (frame) =>
                  frame.endTimestampMs > visibleRange.startTimelineMs &&
                  frame.frameTimestampMs < visibleRange.endTimelineMs,
              )}
            onSelectVisualFrame={onSelectVisualFrame}
          />
        ) : null}
        {track.kind === 'video' ? (
          <TimelineProcessingOverlay
            ranges={processingRanges}
            trackLabel={track.label}
            video={track.kind === 'video'}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
            visibleRange={visibleRange}
          />
        ) : curtain && curtainMotion.visible ? (
          <TranscriptionCurtain
            curtain={curtain}
            revealOffsetMs={curtainMotion.revealOffsetMs}
            trackLabel={track.label}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
            visibleRange={visibleRange}
          />
        ) : null}
      </div>
    </>
  );
}
