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
import { Button } from '@/components/ui/button';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import { TooltipProvider } from '@/components/ui/tooltip';
import { playbackRunView } from './playback-run-view';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import { studioRecallEvidenceAvailable } from './studio-recall-navigation';
import { StudioSidebar } from './studio-sidebar';
import { StudioSourcePlayer } from './studio-source-player';
import { useStudioTimelineStore } from './studio-timeline-store';
import { buildStudioVisualFrames } from './studio-visual-observations';
import { TrackTimeline } from './track-timeline';
import { useRecordingPlaybackController } from './use-recording-playback-controller';
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
  ocrFrames = [],
  initialEvidence = null,
  initialSeek = null,
}: RecordingStudioProps) {
  const [compact, setCompact] = useState(() => window.innerWidth < 1200);
  const [inspectorOpenPreference, setInspectorOpen] = useState<boolean | null>(
    null,
  );
  const inspectorOpen =
    inspectorOpenPreference ??
    (initialEvidence !== null &&
      studioRecallEvidenceAvailable(initialEvidence, detail));
  const inspectorRef = useRef<HTMLDivElement>(null);
  const inspectorTriggerRef = useRef<HTMLButtonElement>(null);
  const interactedOutsideInspector = useRef(false);
  const handleInspectorOpen = useCallback(
    (open: boolean) => {
      if (open === inspectorOpen) return;
      setInspectorOpen(open);
      const restoreTrigger = !interactedOutsideInspector.current;
      interactedOutsideInspector.current = false;
      requestAnimationFrame(() => {
        if (open) {
          inspectorRef.current?.focus();
          interactedOutsideInspector.current = false;
        } else if (restoreTrigger) inspectorTriggerRef.current?.focus();
      });
    },
    [inspectorOpen],
  );
  const handleInspectorInteractOutside = useCallback(
    (event: Event) => {
      if (!inspectorOpen) event.preventDefault();
      else interactedOutsideInspector.current = true;
    },
    [inspectorOpen],
  );
  const preventClosedAutoFocus = useCallback(
    (event: Event) => {
      if (!inspectorOpen) event.preventDefault();
    },
    [inspectorOpen],
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
  const handleClearInspectorSelection = useCallback(() => {
    setInspectorSelectionState(null);
    setDismissedEvidenceId(initialEvidence?.evidenceId ?? null);
  }, [initialEvidence?.evidenceId]);
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

  useEffect(() => {
    const query = window.matchMedia('(max-width: 1199px)');
    function updateLayout() {
      setCompact(query.matches);
    }
    query.addEventListener('change', updateLayout);
    return () => query.removeEventListener('change', updateLayout);
  }, []);

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
      activity={activity}
      aiProcessingState={aiProcessingState}
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
        onClearActivitySelection: handleClearInspectorSelection,
        onCreateRedactions,
        onDeleteRedaction,
        onSeek: handleInspectorSeek,
        onUnredactRanges,
      }}
      model={{
        aiProcessingState,
        debugEnabled,
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
      <div className="flex min-h-[40rem] w-full flex-1 flex-col gap-6 px-5 pb-6 lg:px-8">
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
        {compact ? (
          <>
            <div className="flex justify-end">
              <Sheet
                modal={false}
                open={inspectorOpen}
                onOpenChange={handleInspectorOpen}
              >
                <SheetTrigger asChild>
                  <Button ref={inspectorTriggerRef} variant="outline">
                    Inspector
                  </Button>
                </SheetTrigger>
                <SheetContent
                  hidden={!inspectorOpen}
                  ref={inspectorRef}
                  onOpenAutoFocus={preventClosedAutoFocus}
                  onInteractOutside={handleInspectorInteractOutside}
                  forceMount
                  className="w-full max-w-[28rem] gap-0 p-0 data-[state=closed]:hidden sm:max-w-[28rem]"
                >
                  <SheetHeader className="p-6">
                    <SheetTitle>Recording inspector</SheetTitle>
                    <SheetDescription>
                      Inspect sources, edit annotations and manage redactions.
                    </SheetDescription>
                  </SheetHeader>
                  <div className="min-h-0 flex-1">{inspector}</div>
                </SheetContent>
              </Sheet>
            </div>
            <div className="rounded-workspace min-h-[24rem] overflow-hidden">
              {player}
            </div>
            <div className="rounded-panel min-h-[20rem] flex-1 overflow-hidden border">
              {timeline}
            </div>
          </>
        ) : (
          <ResizablePanelGroup
            className="min-h-0 flex-1"
            orientation="horizontal"
          >
            <ResizablePanel defaultSize="68%" minSize="45%">
              <ResizablePanelGroup orientation="vertical">
                <ResizablePanel defaultSize="60%" minSize={260}>
                  <div className="rounded-workspace h-full overflow-hidden">
                    {player}
                  </div>
                </ResizablePanel>
                <ResizableHandle
                  className="w-6 bg-transparent aria-[orientation=horizontal]:h-6"
                  withHandle
                />
                <ResizablePanel defaultSize="40%" minSize={240}>
                  <div className="rounded-panel h-full overflow-hidden border">
                    {timeline}
                  </div>
                </ResizablePanel>
              </ResizablePanelGroup>
            </ResizablePanel>
            <ResizableHandle
              className="w-6 bg-transparent aria-[orientation=horizontal]:h-6"
              withHandle
            />
            <ResizablePanel defaultSize="32%" minSize={320}>
              {inspector}
            </ResizablePanel>
          </ResizablePanelGroup>
        )}
      </div>
    </TooltipProvider>
  );
}
