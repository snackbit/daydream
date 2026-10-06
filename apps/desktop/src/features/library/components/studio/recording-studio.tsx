import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import type {
  CreateLibraryRedactionRangeInput,
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  RecordingActivity,
  RecordingAiProcessingState,
  RecordingVisualObservations,
  OcrTimelineFrame,
} from '@daydream/shared';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { TooltipProvider } from '@/components/ui/tooltip';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import { StudioSidebar } from './studio-sidebar';
import { StudioSourcePlayer } from './studio-source-player';
import { useStudioTimelineStore } from './studio-timeline-store';
import { buildStudioVisualFrames } from './studio-visual-observations';
import { TrackTimeline } from './track-timeline';
import { useRecordingPlaybackController } from './use-recording-playback-controller';
import { useStudioFrameNavigation } from './use-studio-frame-navigation';
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
}: RecordingStudioProps) {
  const layoutRef = useRef<HTMLDivElement>(null);
  const [compact, setCompact] = useState(false);
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
  const sources = useStudioVideoSources(
    detail,
    playbackAssets,
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
  const seekTimeline = playback.actions.seekTimeline;
  const handleClearInspectorSelection = useCallback(() => {
    setInspectorSelectionState(null);
  }, []);
  const handleInspectActivity = useCallback(
    (selection: StudioInspectorSelection) => {
      setInspectorSelectionState({
        recordingId: detail.id,
        redactionSignature,
        selection,
      });
    },
    [detail.id, redactionSignature],
  );
  const handleInspectTranscript = useCallback(
    (selection: StudioInspectorSelection) => {
      setInspectorSelectionState({
        recordingId: detail.id,
        redactionSignature,
        selection,
      });
    },
    [detail.id, redactionSignature],
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
    initializeTimeline(detail);
  }, [detail, initializeTimeline]);

  useLayoutEffect(() => {
    const element = layoutRef.current;
    if (!element) {
      return;
    }

    const updateLayout = (width: number) => setCompact(width < 896);
    updateLayout(element.getBoundingClientRect().width);

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) {
        updateLayout(entry.contentRect.width);
      }
    });
    observer.observe(element);

    return () => observer.disconnect();
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

  return (
    <TooltipProvider delayDuration={300}>
      <div className="flex h-full min-h-0 w-full flex-1" ref={layoutRef}>
        {frameNavigation.error ? (
          <div
            role="alert"
            className="bg-background text-destructive absolute z-50 p-2 text-xs"
          >
            {frameNavigation.error}
          </div>
        ) : null}
        {compact ? (
          <Tabs
            className="flex min-h-0 flex-1 flex-col gap-0 overflow-y-auto"
            defaultValue="timeline"
          >
            <div className="min-h-[12rem] flex-[3]">
              <StudioSourcePlayer
                sources={sources}
                detail={detail}
                onRefreshPlayback={onRefreshPlayback}
                playback={playback}
                assets={playbackAssets}
              />
            </div>
            <TabsList
              aria-label="Recording studio panels"
              className="grid h-auto w-full shrink-0 grid-cols-2 rounded-none border-y p-1"
            >
              <TabsTrigger className="h-9 px-2 text-xs" value="timeline">
                Timeline
              </TabsTrigger>
              <TabsTrigger className="h-9 px-2 text-xs" value="inspector">
                Inspector
              </TabsTrigger>
            </TabsList>
            <TabsContent
              className="flex min-h-[12rem] flex-[2] flex-col"
              value="timeline"
            >
              <TrackTimeline
                onRefreshRedactions={onRefreshPlayback}
                activity={activity}
                aiProcessingState={aiProcessingState}
                detail={detail}
                mutedTrackIds={mutedTrackIds}
                playback={playback}
                playbackAssets={playbackAssets}
                thumbnailStatus={thumbnailStatus}
                visualFrames={visualFrames}
                onCreateRedactions={onCreateRedactions}
                onClearInspectorSelection={handleClearInspectorSelection}
                onInspectActivity={handleInspectActivity}
                onInspectTranscript={handleInspectTranscript}
                onUnredactRanges={onUnredactRanges}
                onToggleTrackMuted={toggleTrackMuted}
              />
            </TabsContent>
            <TabsContent
              className="flex min-h-[12rem] flex-[2] flex-col"
              value="inspector"
            >
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
                }}
              />
            </TabsContent>
          </Tabs>
        ) : (
          <ResizablePanelGroup
            className="min-h-0 flex-1"
            orientation="horizontal"
          >
            <ResizablePanel defaultSize={74} minSize={50}>
              <ResizablePanelGroup orientation="vertical">
                <ResizablePanel defaultSize={68} minSize={45}>
                  <StudioSourcePlayer
                    sources={sources}
                    detail={detail}
                    onRefreshPlayback={onRefreshPlayback}
                    playback={playback}
                    assets={playbackAssets}
                  />
                </ResizablePanel>
                <ResizableHandle withHandle />
                <ResizablePanel defaultSize={32} minSize={18}>
                  <TrackTimeline
                    onRefreshRedactions={onRefreshPlayback}
                    activity={activity}
                    aiProcessingState={aiProcessingState}
                    detail={detail}
                    mutedTrackIds={mutedTrackIds}
                    playback={playback}
                    playbackAssets={playbackAssets}
                    thumbnailStatus={thumbnailStatus}
                    visualFrames={visualFrames}
                    onCreateRedactions={onCreateRedactions}
                    onClearInspectorSelection={handleClearInspectorSelection}
                    onInspectActivity={handleInspectActivity}
                    onInspectTranscript={handleInspectTranscript}
                    onUnredactRanges={onUnredactRanges}
                    onToggleTrackMuted={toggleTrackMuted}
                  />
                </ResizablePanel>
              </ResizablePanelGroup>
            </ResizablePanel>
            <ResizableHandle withHandle />
            <ResizablePanel defaultSize={26} minSize={18}>
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
                }}
              />
            </ResizablePanel>
          </ResizablePanelGroup>
        )}
      </div>
    </TooltipProvider>
  );
}
