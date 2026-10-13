import {
  useCallback,
  useEffect,
  useState,
  type PointerEvent as ReactPointerEvent,
} from 'react';
import type {
  CreateLibraryRedactionRangeInput,
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  RecordingActivity,
  RecordingAiProcessingState,
  ReprocessingTarget,
} from '@daydream/shared';
import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import type {
  getRecordingWaveformTiles,
  resizeLibraryRedaction,
} from '../../library-client';
import { timelineDurationMs } from '../../library-timeline';
import { GroupedRedactions } from './redactions/grouped-redactions';
import { useSelectionRedactionMutation } from './redactions/use-redaction-mutation';
import { ReprocessingDropdown } from './reprocessing/reprocessing-menu';
import { reprocessingTargets } from './reprocessing/reprocessing-targets';
import { TimelineReprocessingContextMenu } from './reprocessing/timeline-reprocessing-context-menu';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import { useStudioResultWindowStore } from './studio-result-window-store';
import { useStudioTimelineStore } from './studio-timeline-store';
import type { StudioVisualFrame } from './studio-visual-observations';
import type { TimelinePlannedScope } from './timeline/processing/timeline-processing-ranges';
import { TimelineGaps } from './timeline/timeline-gaps';
import { TimelinePlayhead } from './timeline/timeline-playhead';
import { TrackTimelineLanes } from './timeline/track-timeline-lanes';
import type { TranscriptPreviewReader } from './timeline/transcripts/transcript-group-preview';
import {
  TRACK_LABEL_WIDTH_PX,
  TRACK_RULER_HEIGHT_PX,
  TRACK_SELECTION_DRAG_THRESHOLD_PX,
  TRACK_ZOOM_STEP,
} from './track-timeline-math';
import {
  TimelineRangeLabels,
  TimeRuler,
  TrackTimelineToolbar,
} from './track-timeline-parts';
import { useTrackTimelineViewModel } from './track-timeline-view-model';
import type { RecordingPlaybackController } from './use-recording-playback-controller';
import { useTrackTimelineSelection } from './use-track-timeline-selection';
import {
  useTrackTimelinePlayheadFollow,
  useTrackTimelineViewport,
} from './use-track-timeline-viewport';
import { useTrackTimelineZoomScroll } from './use-track-timeline-zoom-scroll';
import { useWaveformTileQueue } from './use-waveform-tile-queue';

export function TrackTimeline({
  timelineOriginMs,
  readWaveformTiles,
  onResizeRedaction,
  activity,
  aiProcessingState,
  plannedProcessingScopes,
  detail,
  mutedTrackIds,
  onCreateRedactions,
  onClearInspectorSelection,
  onInspectActivity,
  onInspectTranscript,
  onUnredactRanges,
  onToggleTrackMuted,
  playback,
  playbackAssets,
  thumbnailStatus,
  visualFrames,
  onRefreshRedactions,
  onReprocess,
  onAudioCleanup,
  suspendFollow = false,
  readTranscriptPreview,
  transcriptPreviewRevision,
  timelinePendingRanges,
  timelineFailedRanges,
  transcriptReadStatus = 'ready',
}: {
  onAudioCleanup?: (() => void) | undefined;
  timelineOriginMs?: number | undefined;
  readWaveformTiles?: typeof getRecordingWaveformTiles | undefined;
  onResizeRedaction?: typeof resizeLibraryRedaction | undefined;
  activity: RecordingActivity | null;
  aiProcessingState: RecordingAiProcessingState | null;
  plannedProcessingScopes?: readonly TimelinePlannedScope[];
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onClearInspectorSelection: () => void;
  onInspectActivity: (selection: StudioInspectorSelection) => void;
  onInspectTranscript: (selection: StudioInspectorSelection) => void;
  onUnredactRanges: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onToggleTrackMuted: (trackId: string) => void;
  playback: RecordingPlaybackController;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
  visualFrames: StudioVisualFrame[];
  onRefreshRedactions: () => void;
  onReprocess?: ((targets: ReprocessingTarget[]) => void) | undefined;
  suspendFollow?: boolean;
  readTranscriptPreview?: TranscriptPreviewReader | undefined;
  transcriptPreviewRevision?: string | undefined;
  timelinePendingRanges?:
    readonly { startOffsetMs: number; endOffsetMs: number }[] | undefined;
  timelineFailedRanges?:
    readonly { startOffsetMs: number; endOffsetMs: number }[] | undefined;
  transcriptReadStatus?: 'loading' | 'error' | 'ready';
}) {
  const { actions, state } = playback;
  const clipsById = useStudioTimelineStore((store) => store.clipsById);
  const clearSelection = useStudioTimelineStore(
    (store) => store.clearSelection,
  );
  const laneOrder = useStudioTimelineStore((store) => store.laneOrder);
  const lanesById = useStudioTimelineStore((store) => store.lanesById);
  const moveLane = useStudioTimelineStore((store) => store.moveLane);
  const selectClip = useStudioTimelineStore((store) => store.selectClip);
  const selectLane = useStudioTimelineStore((store) => store.selectLane);
  const selectRedaction = useStudioTimelineStore(
    (store) => store.selectRedaction,
  );
  const selectVisualFrame = useStudioTimelineStore(
    (store) => store.selectVisualFrame,
  );
  const selection = useStudioTimelineStore((store) => store.selection);
  const redactionMutation = useSelectionRedactionMutation({
    ranges: selection.ranges,
    onCreate: onCreateRedactions,
    onUnredact: onUnredactRanges,
    onClear: clearSelection,
  });
  const setSelection = useStudioTimelineStore((store) => store.setSelection);
  const totalDurationMs = Math.max(1, timelineDurationMs(detail.segments));
  const {
    contentRef,
    followPlayhead,
    keepPlayheadVisible,
    markProgrammaticScroll,
    scrollLeft,
    setFollowPlayhead,
    setScrollLeft,
    setVisibleWindowMs,
    viewportRef,
    viewportWidth,
    visibleWindowMs,
  } = useTrackTimelineViewport({
    detailId: detail.id,
    isPlaying: state.snapshot.isPlaying,
    totalDurationMs,
  });
  const viewModel = useTrackTimelineViewModel({
    detail,
    displayedTimelineMs: state.snapshot.displayedTimelineMs,
    laneOrder,
    lanesById,
    scrollLeft,
    selection,
    viewportWidth,
    visibleWindowMs,
  });
  const hasTimeRangeSelection = selection.ranges.length > 0;
  const publishWindow = useStudioResultWindowStore((store) => store.setWindow);
  useEffect(() => {
    publishWindow(
      detail.id,
      viewModel.waveformRequestRange.startTimelineMs,
      viewModel.waveformRequestRange.endTimelineMs,
      viewModel.timelineWidthPx / totalDurationMs,
    );
  }, [
    detail.id,
    viewModel.waveformRequestRange,
    viewModel.timelineWidthPx,
    totalDurationMs,
    publishWindow,
  ]);
  const targets = reprocessingTargets(
    selection.ranges,
    new Set(detail.tracks.map((track) => track.id)),
  );
  const follow = useCallback(
    () => setFollowPlayhead(true),
    [setFollowPlayhead],
  );
  const [menuIsOpen, menuOpen] = useState(false);
  const { waveformStatus, waveformTiles } = useWaveformTileQueue({
    readTiles: readWaveformTiles,
    detail,
    requestRange: viewModel.waveformRequestRange,
    timelineWidthPx: viewModel.timelineWidthPx,
    totalDurationMs: viewModel.totalDurationMs,
  });
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: TRACK_SELECTION_DRAG_THRESHOLD_PX },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );
  const {
    handleTimelinePointerDown: handleSelectionPointerDown,
    handleTimelinePointerEnd: handleSelectionPointerEnd,
    handleTimelinePointerMove,
    timelineMsFromClientX,
    handleTimelineClickCapture,
  } = useTrackTimelineSelection({
    actions,
    activeLaneIds: viewModel.activeLaneIds,
    activeLaneLayouts: viewModel.activeLaneLayouts,
    clearSelection,
    contentRef,
    selection,
    setFollowPlayhead,
    setSelection,
    timelineWidthPx: viewModel.timelineWidthPx,
    totalDurationMs: viewModel.totalDurationMs,
  });
  const readPlaybackTimeMs = actions.readPlaybackTimeMs;
  const readPlayheadLeftPx = useCallback(
    () =>
      TRACK_LABEL_WIDTH_PX +
      (readPlaybackTimeMs() / totalDurationMs) * viewModel.timelineWidthPx,
    [readPlaybackTimeMs, totalDurationMs, viewModel.timelineWidthPx],
  );
  useTrackTimelinePlayheadFollow({
    readPlayheadLeftPx,
    suspended: suspendFollow || menuIsOpen,
    followPlayhead,
    isPlaying: state.snapshot.isPlaying,
    keepPlayheadVisible,
    playheadLeftPx: viewModel.playheadLeftPx,
    seekRevision: state.snapshot.seekRevision,
    setFollowPlayhead,
  });
  const { zoomVisibleWindow, zoomRange } = useTrackTimelineZoomScroll({
    scrollLeft,
    contentRef,
    displayedTimelineMs: state.snapshot.displayedTimelineMs,
    isPlaying: state.snapshot.isPlaying,
    markProgrammaticScroll,
    setFollowPlayhead,
    setScrollLeft,
    setVisibleWindowMs,
    timelineMsFromClientX,
    timelineViewportWidth: viewModel.timelineViewportWidth,
    totalDurationMs: viewModel.totalDurationMs,
    viewportRef,
    visibleWindowMs,
  });
  const zoomSelection = useCallback(() => {
    if (!selection.ranges.length) return;
    zoomRange(
      Math.min(...selection.ranges.map((range) => range.startMs)),
      Math.max(...selection.ranges.map((range) => range.endMs)),
    );
  }, [selection.ranges, zoomRange]);
  const zoomIn = useCallback(
    () => zoomVisibleWindow(visibleWindowMs / TRACK_ZOOM_STEP),
    [visibleWindowMs, zoomVisibleWindow],
  );
  const zoomOut = useCallback(
    () => zoomVisibleWindow(visibleWindowMs * TRACK_ZOOM_STEP),
    [visibleWindowMs, zoomVisibleWindow],
  );

  function handleDragEnd(event: DragEndEvent) {
    if (!event.over) {
      return;
    }
    onClearInspectorSelection();
    moveLane(String(event.active.id), String(event.over.id));
  }

  function handleTimelinePointerDown(event: ReactPointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return;
    if (
      !(event.target instanceof Element) ||
      !event.target.closest('[data-library-action]')
    ) {
      onClearInspectorSelection();
    }
    handleSelectionPointerDown(event);
  }

  function handleTimelinePointerEnd(event: ReactPointerEvent<HTMLDivElement>) {
    handleSelectionPointerEnd(event);
  }

  function handleSelectClip(clipId: string, additive: boolean) {
    onClearInspectorSelection();
    selectClip(clipId, additive);
  }

  function handleSelectLane(trackId: string, additive: boolean) {
    onClearInspectorSelection();
    selectLane(trackId, additive);
  }

  function handleSelectRedaction(redactionId: string, additive: boolean) {
    onClearInspectorSelection();
    selectRedaction(redactionId, additive);
  }

  function selectVisualFrameForInspection(
    frame: StudioVisualFrame,
    additive: boolean,
  ) {
    onClearInspectorSelection();
    selectVisualFrame(frame.id, additive);
    actions.seekTimeline(frame.frameTimestampMs, { resume: false });
    setFollowPlayhead(true);
  }

  function handleInspectActivity(selection: StudioInspectorSelection) {
    onInspectActivity(selection);
    actions.seekTimeline(selection.timelineOffsetMs, { resume: false });
    setFollowPlayhead(true);
  }

  function selectTranscriptForInspection(selection: StudioInspectorSelection) {
    onInspectTranscript(selection);
    actions.seekTimeline(selection.timelineOffsetMs, { resume: false });
    setFollowPlayhead(true);
  }

  return (
    <div className="bg-background flex h-full flex-col">
      <TrackTimelineToolbar
        followAction={
          !followPlayhead ? (
            <Button size="sm" variant="outline" onClick={follow}>
              Follow playback
            </Button>
          ) : null
        }
        selectionActions={
          hasTimeRangeSelection ? (
            <>
              <Button size="sm" variant="outline" onClick={zoomSelection}>
                Zoom In
              </Button>
              {onReprocess ? (
                <ReprocessingDropdown
                  label="Selection"
                  targets={targets}
                  onReprocess={onReprocess}
                  onOpenChange={menuOpen}
                />
              ) : null}
            </>
          ) : null
        }
        canZoomIn={viewModel.canZoomIn}
        canZoomOut={viewModel.canZoomOut}
        hasTimeRangeSelection={hasTimeRangeSelection}
        selectedRangeRedactionCount={viewModel.selectedRangeRedactions.length}
        statusText={
          waveformTiles?.error ??
          (waveformStatus === 'error'
            ? 'Waveforms unavailable. Retry in Processing.'
            : waveformStatus === 'loading'
              ? 'Preparing waveforms…'
              : 'Video and audio timeline')
        }
        zoomPercent={viewModel.zoomPercent}
        onZoomIn={zoomIn}
        onZoomOut={zoomOut}
        onRedactSelection={redactionMutation.redact}
        onUnredactSelection={redactionMutation.unredact}
        redactionPending={redactionMutation.pending}
      />
      {redactionMutation.error ? (
        <Alert variant="destructive">
          <AlertDescription>{redactionMutation.error}</AlertDescription>
        </Alert>
      ) : null}
      {redactionMutation.pending ? (
        <p role="status" className="text-muted-foreground px-6 py-2 text-xs">
          Updating redactions…
        </p>
      ) : null}
      <ScrollArea
        className="isolate min-h-0 flex-1"
        scrollbars="both"
        viewportRef={viewportRef}
      >
        <TimelineReprocessingContextMenu
          contentRef={contentRef}
          ranges={selection.ranges}
          tracks={detail.tracks}
          timelineWidthPx={viewModel.timelineWidthPx}
          totalDurationMs={viewModel.totalDurationMs}
          onReprocess={onReprocess}
          onOpenChange={menuOpen}
          onZoomSelection={zoomSelection}
        >
          <div
            ref={contentRef}
            aria-label="Track timeline"
            className="relative cursor-crosshair"
            role="group"
            style={{
              width: TRACK_LABEL_WIDTH_PX + viewModel.timelineWidthPx,
            }}
            onPointerCancel={handleTimelinePointerEnd}
            onClickCapture={handleTimelineClickCapture}
            onPointerDown={handleTimelinePointerDown}
            onPointerMove={handleTimelinePointerMove}
            onPointerUp={handleTimelinePointerEnd}
          >
            <TimelinePlayhead
              time={state.snapshot.displayedTimelineMs}
              readTime={actions.readPlaybackTimeMs}
              playing={state.snapshot.isPlaying}
              width={viewModel.timelineWidthPx}
              duration={totalDurationMs}
              follow={followPlayhead && !suspendFollow && !menuIsOpen}
              keepVisible={keepPlayheadVisible}
            />
            <TimelineRangeLabels
              ranges={selection.ranges}
              timelineWidthPx={viewModel.timelineWidthPx}
              totalDurationMs={viewModel.totalDurationMs}
            />
            <GroupedRedactions
              onResize={onResizeRedaction}
              redactions={detail.redactions}
              lanes={viewModel.activeLaneLayouts}
              selectedIds={viewModel.selectedRedactionIds}
              timelineWidthPx={viewModel.timelineWidthPx}
              totalDurationMs={totalDurationMs}
              onSelect={handleSelectRedaction}
              onSaved={onRefreshRedactions}
            />
            <TimelineGaps
              segments={detail.segments}
              timelineWidthPx={viewModel.timelineWidthPx}
              totalDurationMs={totalDurationMs}
            />
            <div
              className="grid"
              style={{
                gridTemplateColumns: `${TRACK_LABEL_WIDTH_PX}px ${viewModel.timelineWidthPx}px`,
              }}
            >
              <div
                className="bg-background sticky left-0 z-60 border-r border-b px-3"
                style={{ height: TRACK_RULER_HEIGHT_PX }}
              />
              <TimeRuler
                originMs={timelineOriginMs}
                ticks={viewModel.rulerTicks}
                timelineWidthPx={viewModel.timelineWidthPx}
              />
            </div>
            <DndContext
              collisionDetection={closestCenter}
              sensors={sensors}
              onDragEnd={handleDragEnd}
            >
              <SortableContext
                items={viewModel.activeLaneIds}
                strategy={verticalListSortingStrategy}
              >
                <TrackTimelineLanes
                  view={viewModel}
                  lanesById={lanesById}
                  media={{
                    readTranscriptPreview,
                    transcriptPreviewRevision,
                    timelinePendingRanges:
                      timelinePendingRanges ??
                      (transcriptReadStatus === 'loading'
                        ? [
                            {
                              startOffsetMs:
                                viewModel.paintRange.startTimelineMs,
                              endOffsetMs: viewModel.paintRange.endTimelineMs,
                            },
                          ]
                        : []),
                    timelineFailedRanges:
                      timelineFailedRanges ??
                      (transcriptReadStatus === 'error'
                        ? [
                            {
                              startOffsetMs:
                                viewModel.paintRange.startTimelineMs,
                              endOffsetMs: viewModel.paintRange.endTimelineMs,
                            },
                          ]
                        : []),
                    visibleRange: viewModel.paintRange,
                    aiProcessingState,
                    plannedProcessingScopes,
                    clipDrafts: clipsById,
                    detail,
                    mutedTrackIds,
                    playbackAssets,
                    selectedClipIds: viewModel.selectedClipIds,
                    selectedRedactionIds: viewModel.selectedRedactionIds,
                    selectedVisualFrameIds: viewModel.selectedVisualFrameIds,
                    thumbnailStatus,
                    timelineWidthPx: viewModel.timelineWidthPx,
                    totalDurationMs: viewModel.totalDurationMs,
                    waveformTiles,
                    visualFrames,
                    onInspectTranscript: selectTranscriptForInspection,
                    onSelectClip: handleSelectClip,
                    onSelectLane: handleSelectLane,
                    onSelectRedaction: handleSelectRedaction,
                    onSelectVisualFrame: selectVisualFrameForInspection,
                    onToggleTrackMuted,
                    onReprocess,
                    onAudioCleanup,
                    onZoomRange: zoomRange,
                  }}
                  activity={{
                    overview: aiProcessingState?.timelineOverview,
                    visibleRange: viewModel.paintRange,
                    onZoomRange: zoomRange,
                    activity,
                    segments: detail.segments,
                    redactions: detail.redactions,
                    selectedRedactionIds: viewModel.selectedRedactionIds,
                    timelineWidthPx: viewModel.timelineWidthPx,
                    totalDurationMs: viewModel.totalDurationMs,
                    onInspectActivity: handleInspectActivity,
                    onSelectLane: handleSelectLane,
                    onSelectRedaction: handleSelectRedaction,
                  }}
                />
              </SortableContext>
            </DndContext>
          </div>
        </TimelineReprocessingContextMenu>
      </ScrollArea>
    </div>
  );
}
