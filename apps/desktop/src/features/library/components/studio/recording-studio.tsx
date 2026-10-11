import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  CreateLibraryRedactionRangeInput,
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  RecordingActivity,
  RecordingAiProcessingState,
  RecordingVisualObservations,
  OcrTimelineFrame,
  RecallEvidence,
} from '@daydream/shared';
import { TooltipProvider } from '@/components/ui/tooltip';
import { useStudioSelectionDismissal } from './interactions/use-studio-selection-dismissal';
import { RecordingStudioLayout } from './layout/recording-studio-layout';
import { playbackRunView } from './playback-run-view';
import { ReprocessingDialog } from './reprocessing/reprocessing-dialog';
import { ReprocessingStatus } from './reprocessing/reprocessing-status';
import { useStudioReprocessing } from './reprocessing/use-studio-reprocessing';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import { studioRecallEvidenceAvailable } from './studio-recall-navigation';
import { StudioSidebar } from './studio-sidebar';
import { StudioSourcePlayer } from './studio-source-player';
import { useStudioTimelineStore } from './studio-timeline-store';
import { buildStudioVisualFrames } from './studio-visual-observations';
import { TrackTimeline } from './track-timeline';
import { useRecordingPlaybackController } from './use-recording-playback-controller';
import { useStudioEntryNavigation } from './use-studio-entry-navigation';
import { useStudioFrameNavigation } from './use-studio-frame-navigation';
import { useStudioRecallNavigation } from './use-studio-recall-navigation';
import { useStudioSpacePlayback } from './use-studio-space-playback';
import { useStudioVideoSources } from './use-studio-video-sources';

export interface RecordingStudioProps {
  debugEnabled?: boolean;
  activity: RecordingActivity | null;
  aiProcessingState: RecordingAiProcessingState | null;
  detail: LibraryRecordingDetail;
  onRefreshPlayback: () => void;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onDeleteRedaction: (redactionId: string) => Promise<void>;
  onUnredactRanges: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
  visualObservations: RecordingVisualObservations | null;
  ocrFrames?: OcrTimelineFrame[];
  initialEvidence?: RecallEvidence | null;
  initialSeek?: number | null;
}

const EMPTY_OCR_FRAMES: OcrTimelineFrame[] = [];

export function RecordingStudio({
  debugEnabled = false,
  activity,
  aiProcessingState,
  detail,
  onRefreshPlayback,
  onCreateRedactions,
  onDeleteRedaction,
  onUnredactRanges,
  playbackAssets,
  thumbnailStatus,
  visualObservations,
  ocrFrames = EMPTY_OCR_FRAMES,
  initialEvidence = null,
  initialSeek = null,
}: RecordingStudioProps) {
  const reprocessing = useStudioReprocessing(
    detail.id,
    aiProcessingState?.job ?? null,
  );
  const [dismissedEvidenceId, setDismissedEvidenceId] = useState<string | null>(
    null,
  );
  const [mutedTrackIds, setMutedTrackIds] = useState<Set<string>>(
    () => new Set(),
  );
  const [inspectorSelectionState, setInspectorSelectionState] = useState<{
    recordingId: string;
    redactionSignature: string;
    selection: StudioInspectorSelection;
  } | null>(null);
  const [sidebarResetRevision, setSidebarResetRevision] = useState(0);
  const initializeTimeline = useStudioTimelineStore(
    (state) => state.initializeFromDetail,
  );
  const visualFrames = useMemo(
    () => buildStudioVisualFrames(visualObservations, detail, ocrFrames),
    [ocrFrames, detail, visualObservations],
  );
  const inspectedIds = useStudioTimelineStore(
    (state) => state.selection.visualFrameIds,
  );
  const inspectedSource = visualFrames.find(
    (frame) => frame.id === inspectedIds.at(-1),
  )?.sourceTrackId;
  const runView = useMemo(
    () => playbackRunView(detail, playbackAssets),
    [detail, playbackAssets],
  );
  const sources = useStudioVideoSources(
    runView.detail,
    runView.assets,
    inspectedSource,
  );
  const playback = useRecordingPlaybackController({
    detail: sources.selectedDetail,
    mutedTrackIds,
    playbackAssets: sources.selectedAssets,
    autoSkipRedacted: !sources.grid,
  });
  const redactionSignature = useMemo(
    () =>
      detail.redactions
        .map(
          (redaction) =>
            `${redaction.id}:${redaction.startOffsetMs}:${redaction.endOffsetMs}:${redaction.trackIds.join(',')}`,
        )
        .join('|'),
    [detail.redactions],
  );
  const inspectorSelection =
    inspectorSelectionState?.recordingId === detail.id &&
    inspectorSelectionState.redactionSignature === redactionSignature
      ? inspectorSelectionState.selection
      : null;
  const recallEvidence =
    initialEvidence &&
    dismissedEvidenceId !== initialEvidence.evidenceId &&
    studioRecallEvidenceAvailable(initialEvidence, detail)
      ? initialEvidence
      : null;
  const seekTimeline = playback.actions.seekTimeline;
  const initialSeekApplied = useRef<string | null>(null);
  useEffect(() => {
    if (initialSeek === null || initialEvidence !== null || !playbackAssets)
      return;
    const key = `${detail.id}:${initialSeek}`;
    if (initialSeekApplied.current === key) return;
    initialSeekApplied.current = key;
    seekTimeline(initialSeek, { resume: false });
  }, [initialSeek, initialEvidence, playbackAssets, seekTimeline, detail.id]);
  const handleClearInspectorSelection = useCallback(
    (reason?: 'outside' | 'escape') => {
      if (reason === 'escape')
        setSidebarResetRevision((revision) => revision + 1);
      setInspectorSelectionState(null);
      setDismissedEvidenceId(initialEvidence?.evidenceId ?? null);
    },
    [initialEvidence?.evidenceId],
  );
  useStudioSelectionDismissal(handleClearInspectorSelection);
  const handleInspectActivity = useCallback(
    (selection: StudioInspectorSelection) => {
      setDismissedEvidenceId(initialEvidence?.evidenceId ?? null);
      setInspectorSelectionState({
        recordingId: detail.id,
        redactionSignature,
        selection,
      });
    },
    [detail.id, redactionSignature, initialEvidence?.evidenceId],
  );
  const handleInspectTranscript = useCallback(
    (selection: StudioInspectorSelection) => {
      setDismissedEvidenceId(initialEvidence?.evidenceId ?? null);
      setInspectorSelectionState({
        recordingId: detail.id,
        redactionSignature,
        selection,
      });
    },
    [detail.id, redactionSignature, initialEvidence?.evidenceId],
  );
  const handleInspectorSeek = useCallback(
    (timelineOffsetMs: number) => {
      seekTimeline(timelineOffsetMs, { resume: false });
    },
    [seekTimeline],
  );
  const frameNavigation = useStudioFrameNavigation(
    detail.id,
    playback.state.displayedTimelineMs,
    playback.state.seekRevision,
    seekTimeline,
    detail.segments.some((segment) => (segment.videoStreams?.length ?? 0) > 0)
      ? sources.selectedTrackId
      : undefined,
  );
  const handleEntryNavigation = useStudioEntryNavigation(
    detail.id,
    redactionSignature,
    sources,
    playbackAssets !== null,
    handleInspectorSeek,
  );
  useStudioSpacePlayback(playback.actions.togglePlayback, frameNavigation.step);

  useEffect(() => {
    initializeTimeline(runView.detail);
  }, [runView.detail, initializeTimeline]);

  useStudioRecallNavigation({
    evidence: recallEvidence,
    playbackReady: playbackAssets !== null,
    seek: seekTimeline,
    sources,
    visualFrames,
  });

  function toggleTrackMuted(trackId: string) {
    setMutedTrackIds((current) => {
      const next = new Set(current);
      if (next.has(trackId)) {
        next.delete(trackId);
      } else {
        next.add(trackId);
      }

      return next;
    });
  }

  const player = (
    <StudioSourcePlayer
      sources={sources}
      detail={detail}
      onRefreshPlayback={onRefreshPlayback}
      playback={playback}
      assets={runView.assets}
    />
  );
  const timeline = (
    <TrackTimeline
      onRefreshRedactions={onRefreshPlayback}
      onReprocess={reprocessing.open}
      suspendFollow={reprocessing.request !== null}
      activity={activity}
      aiProcessingState={aiProcessingState}
      plannedProcessingScopes={reprocessing.plannedScopes}
      detail={runView.detail}
      mutedTrackIds={mutedTrackIds}
      playback={playback}
      playbackAssets={runView.assets}
      thumbnailStatus={thumbnailStatus}
      visualFrames={visualFrames}
      onCreateRedactions={onCreateRedactions}
      onClearInspectorSelection={handleClearInspectorSelection}
      onInspectActivity={handleInspectActivity}
      onInspectTranscript={handleInspectTranscript}
      onUnredactRanges={onUnredactRanges}
      onToggleTrackMuted={toggleTrackMuted}
    />
  );
  const inspector = (
    <StudioSidebar
      actions={{
        readPlaybackTimeMs: playback.actions.readPlaybackTimeMs,
        onClearActivitySelection: handleClearInspectorSelection,
        onCreateRedactions,
        onDeleteRedaction,
        onSeek: handleInspectorSeek,
        onNavigateEntry: handleEntryNavigation,
        onUnredactRanges,
      }}
      model={{
        seekRevision: playback.state.seekRevision,
        aiProcessingState,
        activity,
        isPlaying: playback.state.snapshot.isPlaying,
        debugEnabled,
        resetRevision: sidebarResetRevision,
        timelineOffsetMs: playback.state.displayedTimelineMs,
        detail,
        selection: inspectorSelection,
        visualFrames,
        recallEvidence,
      }}
    />
  );

  return (
    <TooltipProvider delayDuration={300}>
      <div
        data-studio-root
        className="flex min-h-[40rem] w-full flex-1 flex-col gap-6 px-5 pb-6 lg:px-8"
      >
        <ReprocessingDialog controller={reprocessing} />
        <ReprocessingStatus
          state={aiProcessingState}
          queued={reprocessing.queued}
        />
        {initialEvidence &&
        !studioRecallEvidenceAvailable(initialEvidence, detail) ? (
          <p role="status" className="bg-muted rounded-panel p-6 text-sm">
            This recall source is unavailable. Its media may be missing or its
            range may have been hidden.
          </p>
        ) : null}
        {frameNavigation.error ? (
          <div
            role="alert"
            className="bg-background text-destructive absolute z-50 p-2 text-xs"
          >
            {frameNavigation.error}
          </div>
        ) : null}
        <RecordingStudioLayout
          player={player}
          timeline={timeline}
          inspector={inspector}
          initiallyOpen={recallEvidence !== null}
        />
      </div>
    </TooltipProvider>
  );
}
