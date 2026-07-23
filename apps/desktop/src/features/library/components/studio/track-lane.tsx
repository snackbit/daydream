import {
  useRef,
  type ButtonHTMLAttributes,
  type CSSProperties,
  type MouseEvent,
  type PointerEvent,
} from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryRecordingTrack,
  LibraryWaveformTiles,
  RecordingAiProcessingState,
  VisualFrameProcessingState,
} from '@daydream/shared';
import { cn } from '@/lib/utils';
import { formatDuration } from '../../library-format';
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

type VisualFrameSource = 'mixed' | 'ocr' | 'vlm';

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
  onSelectVisualFrame: (frame: StudioVisualFrame, additive: boolean) => void;
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
  visualFrameStates,
  visualFrames,
  onSelectVisualFrame,
}: {
  selectedVisualFrameIds: Set<string>;
  timelineWidthPx: number;
  totalDurationMs: number;
  visualFrameStates: VisualFrameProcessingState[];
  visualFrames: StudioVisualFrame[];
  onSelectVisualFrame: (frame: StudioVisualFrame, additive: boolean) => void;
}) {
  const processingMarkers = visualProcessingMarkers({
    visualFrameStates,
    visualFrames,
  });
  if (visualFrames.length === 0 && processingMarkers.length === 0) {
    return null;
  }

  return (
    <>
      {processingMarkers.map((marker) => (
        <VisualProcessingMarker
          key={marker.id}
          marker={marker}
          timelineWidthPx={timelineWidthPx}
          totalDurationMs={totalDurationMs}
        />
      ))}
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
        const source = visualFrameSource(frame);
        const title = visualFrameMarkerTitle(frame);

        return (
          <button
            aria-label={title}
            aria-pressed={selected}
            className={cn(
              'absolute bottom-1 z-20 h-6 overflow-hidden rounded-sm border px-1 text-[0.625rem] font-medium shadow-sm',
              'focus-visible:ring-ring hover:brightness-95 focus-visible:ring-2 focus-visible:outline-none',
              visualFrameMarkerClassName(source),
              selected && 'ring-ring ring-2',
            )}
            data-library-action
            data-visual-frame-source={source}
            key={frame.id}
            style={{
              left,
              width,
              ...visualFrameMarkerStyle(source),
            }}
            title={title}
            type="button"
            onClick={(event) => {
              event.stopPropagation();
              onSelectVisualFrame(
                frame,
                event.shiftKey || event.metaKey || event.ctrlKey,
              );
            }}
          >
            <span className="block truncate">
              {formatVisualFrameMarkerTimestamp(frame.frameTimestampMs)}
            </span>
          </button>
        );
      })}
    </>
  );
}

interface VisualProcessingMarkerModel {
  endTimestampMs: number;
  id: string;
  label: string;
  sourceKind: 'ocr' | 'vlm';
  status: VisualFrameProcessingState['status'];
  timestampMs: number;
}

function visualProcessingMarkers({
  visualFrameStates,
  visualFrames,
}: {
  visualFrameStates: VisualFrameProcessingState[];
  visualFrames: StudioVisualFrame[];
}): VisualProcessingMarkerModel[] {
  const outputKeys = new Set<string>();
  for (const frame of visualFrames) {
    if (frame.ocrRegions.length > 0) {
      for (const sourceFrameRef of frame.sourceFrameRefs) {
        outputKeys.add(`${sourceFrameRef}:ocr`);
      }
    }
    if (frame.vlmObservations.length > 0) {
      for (const sourceFrameRef of frame.sourceFrameRefs) {
        outputKeys.add(`${sourceFrameRef}:vlm`);
      }
    }
  }

  return visualFrameStates
    .filter(
      (state) =>
        state.status !== 'succeeded' ||
        state.outputCount === 0 ||
        !outputKeys.has(`${state.sourceFrameRef}:${state.sourceKind}`),
    )
    .map((state) => ({
      endTimestampMs: state.frameTimestampMs + 1,
      id: state.id,
      label: visualProcessingMarkerLabel(state),
      sourceKind: state.sourceKind,
      status: state.status,
      timestampMs: state.frameTimestampMs,
    }))
    .sort(
      (left, right) =>
        left.timestampMs - right.timestampMs ||
        left.sourceKind.localeCompare(right.sourceKind) ||
        left.id.localeCompare(right.id),
    );
}

function VisualProcessingMarker({
  marker,
  timelineWidthPx,
  totalDurationMs,
}: {
  marker: VisualProcessingMarkerModel;
  timelineWidthPx: number;
  totalDurationMs: number;
}) {
  const left =
    (Math.min(marker.timestampMs, totalDurationMs) / totalDurationMs) *
    timelineWidthPx;

  return (
    <div
      aria-label={marker.label}
      className={cn(
        'absolute bottom-8 z-10 h-2 rounded-full border',
        marker.sourceKind === 'ocr' && 'border-amber-500 bg-amber-300',
        marker.sourceKind === 'vlm' && 'border-sky-500 bg-sky-300',
        marker.status === 'queued' && 'opacity-45',
        marker.status === 'running' && 'ai-processing-construction opacity-90',
        marker.status === 'failed' &&
          'border-destructive bg-destructive opacity-90',
        marker.status === 'cancelled' &&
          'border-muted-foreground bg-muted-foreground opacity-60',
        marker.status === 'succeeded' && 'opacity-70',
      )}
      data-library-action
      style={{
        left,
        width: Math.max(4, (1 / totalDurationMs) * timelineWidthPx),
      }}
      title={marker.label}
    />
  );
}

function visualProcessingMarkerLabel(
  state: VisualFrameProcessingState,
): string {
  const kind = state.sourceKind === 'ocr' ? 'OCR' : 'VLM';
  const timestamp = formatVisualFrameMarkerTimestamp(state.frameTimestampMs);
  if (state.status === 'failed' && state.errorCode) {
    return `${kind} frame ${timestamp} failed: ${state.errorCode}`;
  }
  if (state.status === 'succeeded') {
    return `${kind} frame ${timestamp} processed, ${state.outputCount} outputs`;
  }

  return `${kind} frame ${timestamp} ${state.status}`;
}

function visualFrameSource(frame: StudioVisualFrame): VisualFrameSource {
  const hasOcr = frame.ocrRegions.length > 0;
  const hasVlm = frame.vlmObservations.length > 0;
  if (hasOcr && hasVlm) {
    return 'mixed';
  }

  return hasVlm ? 'vlm' : 'ocr';
}

function visualFrameMarkerClassName(source: VisualFrameSource): string {
  switch (source) {
    case 'mixed':
      return 'border-cyan-600/90 bg-amber-100 text-slate-950 dark:border-cyan-300/80';
    case 'vlm':
      return 'border-sky-500/80 bg-sky-100/95 text-sky-950 dark:border-sky-300/80 dark:bg-sky-950/90 dark:text-sky-100';
    case 'ocr':
      return 'border-amber-500/80 bg-amber-100/95 text-amber-950 dark:border-amber-300/80 dark:bg-amber-950/90 dark:text-amber-100';
  }
}

function visualFrameMarkerStyle(
  source: VisualFrameSource,
): CSSProperties | undefined {
  if (source !== 'mixed') {
    return undefined;
  }

  return {
    backgroundImage:
      'repeating-linear-gradient(135deg, rgb(254 243 199 / 0.95) 0, rgb(254 243 199 / 0.95) 6px, rgb(186 230 253 / 0.95) 6px, rgb(186 230 253 / 0.95) 12px)',
  };
}

function formatVisualFrameMarkerTimestamp(timestampMs: number): string {
  if (timestampMs < 60_000 && timestampMs % 1_000 !== 0) {
    return `${(timestampMs / 1_000).toFixed(1)}s`;
  }

  return formatDuration(timestampMs);
}

function visualFrameMarkerTitle(frame: StudioVisualFrame): string {
  const startSeconds = (frame.frameTimestampMs / 1_000).toFixed(1);
  const endSeconds = (frame.endTimestampMs / 1_000).toFixed(1);
  const vlmText =
    frame.vlmObservations.length > 0
      ? `${frame.vlmObservations.length} VLM`
      : 'no VLM';
  const ocrText = `${frame.ocrRegions.length} OCR ${
    frame.ocrRegions.length === 1 ? 'candidate' : 'candidates'
  }`;

  return `Visual frame ${startSeconds}s to ${endSeconds}s, ${vlmText}, ${ocrText}`;
}
