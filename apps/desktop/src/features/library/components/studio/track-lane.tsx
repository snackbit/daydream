import { useMemo, type ButtonHTMLAttributes } from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryRecordingTrack,
  LibraryWaveformTiles,
  RecordingAiProcessingState,
  ReprocessingTarget,
} from '@daydream/shared';
import { getRecordingTranscriptPreview } from '@/features/ai/transcript/preview-client';
import { cn } from '@/lib/utils';
import {
  buildDerivedLane,
  SegmentTranscriptionLayer,
} from './derived-ai-lane-strip';
import { ContinuousTrackMedia } from './media/continuous-track-media';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import {
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
import { SourceOverviewStrip } from './timeline/source-overview-strip';
import { TimelineLaneWindow } from './timeline/timeline-lane-window';
import type { TranscriptPreviewReader } from './timeline/transcripts/transcript-group-preview';
import { TrackLaneHeader } from './track-lane-header';
import { RedactionOverlay, TimeRangeOverlay } from './track-lane-overlays';
import { VisualFrameSegments } from './track-lane-visual-markers';
import { trackLaneHeightPxForKind } from './track-timeline-math';

export interface TrackLaneProps {
  onAudioCleanup?: (() => void) | undefined;
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
  onZoomRange?: ((start: number, end: number) => void) | undefined;
  readTranscriptPreview?: TranscriptPreviewReader | undefined;
  transcriptPreviewRevision?: string | undefined;
  timelinePendingRanges?:
    readonly { startOffsetMs: number; endOffsetMs: number }[] | undefined;
  timelineFailedRanges?:
    readonly { startOffsetMs: number; endOffsetMs: number }[] | undefined;
}

export function TrackLane({
  plannedProcessingScopes,
  visibleRange,
  aiProcessingState,
  detail,
  dragHandleProps,
  mutedTrackIds,
  onInspectTranscript,
  onSelectLane,
  onSelectRedaction,
  onToggleTrackMuted,
  playbackAssets,
  selectedLane,
  selectedRedactionIds,
  selectedVisualFrameIds,
  timelineWidthPx,
  timeRanges,
  totalDurationMs,
  track,
  lane,
  visualFrames,
  waveformTiles,
  onSelectVisualFrame,
  onReprocess,
  onAudioCleanup,
  onZoomRange,
  readTranscriptPreview = getRecordingTranscriptPreview,
  transcriptPreviewRevision,
  timelinePendingRanges,
  timelineFailedRanges,
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
  const waveformTrack =
    waveformTiles?.tracks.find((waveform) => waveform.trackId === track.id) ??
    waveformTiles?.tracks.find((waveform) => waveform.trackId === track.kind);
  const sourceSpan = useMemo(
    () =>
      detail.segments[0]
        ? {
            ...detail.segments[0],
            playbackOffsetMs: 0,
            durationMs: totalDurationMs,
          }
        : null,
    [detail.segments, totalDurationMs],
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
  const overviewRanges = useMemo(
    () =>
      (aiProcessingState?.timelineOverview ?? []).filter(
        (bucket) => bucket.trackId === track.id && bucket.kind === 'speech',
      ),
    [aiProcessingState?.timelineOverview, track.id],
  );
  const previewRevision = `${transcriptPreviewRevision}:${aiProcessingState?.resultsRevision}:${JSON.stringify(detail.redactions)}`;
  const pending = timelinePendingRanges?.some(
    (r) =>
      r.startOffsetMs < visibleRange.endTimelineMs &&
      r.endOffsetMs > visibleRange.startTimelineMs,
  );
  const failed = timelineFailedRanges?.some(
    (r) =>
      r.startOffsetMs < visibleRange.endTimelineMs &&
      r.endOffsetMs > visibleRange.startTimelineMs,
  );

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
        onAudioCleanup={
          track.kind === 'microphone' ? onAudioCleanup : undefined
        }
      />
      <TimelineLaneWindow
        className={cn(
          'bg-muted/60 group-hover/lane:bg-muted/80 relative isolate z-0 cursor-pointer overflow-hidden border-b transition-colors',
          selectedLane && 'bg-accent/20',
        )}
        trackId={track.id}
        height={trackLaneHeightPxForKind(track.kind)}
        width={timelineWidthPx}
        duration={totalDurationMs}
        range={visibleRange}
      >
        <ContinuousTrackMedia
          detail={detail}
          track={track}
          assets={playbackAssets}
          waveform={waveformTrack}
          range={visibleRange}
          width={timelineWidthPx}
          duration={totalDurationMs}
        />
        {audioTrack && sourceSpan ? (
          <SegmentTranscriptionLayer
            lane={derivedLane}
            segment={sourceSpan}
            track={track}
            color={color.strong}
            onInspectTranscript={onInspectTranscript}
            visibleRange={visibleRange}
            timelineWidthPx={timelineWidthPx}
            onZoomRange={onZoomRange}
            overviewRanges={overviewRanges}
            hiddenRanges={curtain?.pendingRanges ?? []}
            publishedRanges={curtain?.publishedRanges ?? []}
            revealOffsetMs={
              curtainMotion.visible ? curtainMotion.revealOffsetMs : null
            }
          />
        ) : null}
        {(aiProcessingState?.timelineOverview !== undefined ||
          (audioTrack && (pending || failed))) &&
        onZoomRange ? (
          <SourceOverviewStrip
            key={previewRevision}
            range={visibleRange}
            buckets={aiProcessingState?.timelineOverview ?? []}
            trackId={track.id}
            width={timelineWidthPx}
            duration={totalDurationMs}
            onZoom={onZoomRange}
            recordingId={detail.id}
            trackLabel={track.label}
            sourceRole={track.kind === 'video' ? undefined : track.kind}
            readPreview={readTranscriptPreview}
            previewRevision={previewRevision}
            pending={pending}
            error={failed}
          />
        ) : null}
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
        {track.kind === 'video' &&
        aiProcessingState?.timelineOverview === undefined ? (
          <VisualFrameSegments
            visualFrameStates={(aiProcessingState?.visualFrames ?? []).filter(
              (frame) =>
                frame.sourceKind === 'vlm' &&
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
                  frame.vlmObservations.length > 0 ||
                  selectedVisualFrameIds.has(frame.id),
              )
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
            onZoomRange={onZoomRange}
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
      </TimelineLaneWindow>
    </>
  );
}
