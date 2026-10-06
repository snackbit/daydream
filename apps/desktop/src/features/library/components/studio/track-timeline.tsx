import type { PointerEvent as ReactPointerEvent } from 'react';
import type {
  CreateLibraryRedactionRangeInput,
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  RecordingActivity,
  RecordingAiProcessingState,
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
import { ScrollArea } from '@/components/ui/scroll-area';
import { timelineDurationMs } from '../../library-timeline';
import { GroupedRedactions } from './redactions/grouped-redactions';
import { isStudioActivityLaneKind } from './studio-activity-lanes';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import { useStudioTimelineStore } from './studio-timeline-store';
import type { StudioVisualFrame } from './studio-visual-observations';
import {
  TRACK_LABEL_WIDTH_PX,
  TRACK_RULER_HEIGHT_PX,
  TRACK_SELECTION_DRAG_THRESHOLD_PX,
  TRACK_ZOOM_STEP,
} from './track-timeline-math';
import {
  SortableTrackLane,
  SortableActivityTrackLane,
  TimelineRangeLabels,
  TimeRuler,
  TrackTimelineToolbar,
} from './track-timeline-parts';
import { redactionRangesFromSelection } from './track-timeline-redaction-actions';
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
  activity,
  aiProcessingState,
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
}: {
  activity: RecordingActivity | null;
  aiProcessingState: RecordingAiProcessingState | null;
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
  const { waveformStatus, waveformTiles } = useWaveformTileQueue({
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
  useTrackTimelinePlayheadFollow({
    followPlayhead,
    isPlaying: state.snapshot.isPlaying,
    keepPlayheadVisible,
    playheadLeftPx: viewModel.playheadLeftPx,
    seekRevision: state.snapshot.seekRevision,
    setFollowPlayhead,
  });
  const { zoomVisibleWindow } = useTrackTimelineZoomScroll({
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

  function handleDragEnd(event: DragEndEvent) {
    if (!event.over) {
      return;
    }
    onClearInspectorSelection();
    moveLane(String(event.active.id), String(event.over.id));
  }

  function handleTimelinePointerDown(event: ReactPointerEvent<HTMLDivElement>) {
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

  async function redactSelection() {
    if (selection.ranges.length === 0) {
      return;
    }
    await onCreateRedactions(redactionRangesFromSelection(selection.ranges));
    clearSelection();
  }

  async function unredactSelection() {
    if (viewModel.selectedRangeRedactions.length === 0) {
      return;
    }
    await onUnredactRanges(redactionRangesFromSelection(selection.ranges));
    clearSelection();
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
        canZoomIn={viewModel.canZoomIn}
        canZoomOut={viewModel.canZoomOut}
        hasTimeRangeSelection={hasTimeRangeSelection}
        selectedRangeRedactionCount={viewModel.selectedRangeRedactions.length}
        statusText={
          waveformTiles?.error ??
          (waveformStatus === 'loading'
            ? 'Loading waveforms...'
            : 'Video and audio timeline')
        }
        zoomPercent={viewModel.zoomPercent}
        onZoomIn={() => zoomVisibleWindow(visibleWindowMs / TRACK_ZOOM_STEP)}
        onZoomOut={() => zoomVisibleWindow(visibleWindowMs * TRACK_ZOOM_STEP)}
        onRedactSelection={() => void redactSelection()}
        onUnredactSelection={() => void unredactSelection()}
      />
      <ScrollArea
        className="min-h-0 flex-1"
        scrollbars="both"
        viewportRef={viewportRef}
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
          onPointerDown={handleTimelinePointerDown}
          onPointerMove={handleTimelinePointerMove}
          onPointerUp={handleTimelinePointerEnd}
        >
          <div
            aria-hidden="true"
            className="bg-primary pointer-events-none absolute top-0 bottom-0 z-30 w-px"
            style={{ left: viewModel.playheadLeftPx }}
          />
          <TimelineRangeLabels
            ranges={selection.ranges}
            timelineWidthPx={viewModel.timelineWidthPx}
            totalDurationMs={viewModel.totalDurationMs}
          />
          <GroupedRedactions
            redactions={detail.redactions}
            lanes={viewModel.activeLaneLayouts}
            selectedIds={viewModel.selectedRedactionIds}
            timelineWidthPx={viewModel.timelineWidthPx}
            totalDurationMs={totalDurationMs}
            onSelect={handleSelectRedaction}
            onSaved={onRefreshRedactions}
          />
          <div
            className="grid"
            style={{
              gridTemplateColumns: `${TRACK_LABEL_WIDTH_PX}px ${viewModel.timelineWidthPx}px`,
            }}
          >
            <div
              className="bg-background sticky left-0 z-40 border-r border-b px-3"
              style={{ height: TRACK_RULER_HEIGHT_PX }}
            />
            <TimeRuler
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
              {viewModel.orderedLaneIds.map((laneId) => {
                const lane = lanesById[laneId];
                if (!lane) {
                  return null;
                }
                if (isStudioActivityLaneKind(lane.kind)) {
                  return (
                    <SortableActivityTrackLane
                      activity={activity}
                      segments={detail.segments}
                      key={lane.trackId}
                      lane={lane}
                      redactions={detail.redactions}
                      selectedLane={viewModel.selectedLaneIds.has(lane.trackId)}
                      selectedRedactionIds={viewModel.selectedRedactionIds}
                      timelineWidthPx={viewModel.timelineWidthPx}
                      timeRanges={
                        viewModel.rangesByLaneId.get(lane.trackId) ?? []
                      }
                      totalDurationMs={viewModel.totalDurationMs}
                      onInspectActivity={handleInspectActivity}
                      onSelectLane={handleSelectLane}
                      onSelectRedaction={handleSelectRedaction}
                    />
                  );
                }
                const track = detail.tracks.find(
                  (candidate) => candidate.id === lane.trackId,
                );
                if (!track) {
                  return null;
                }

                function handleSelectTranscript(
                  transcript: StudioInspectorSelection,
                ) {
                  selectTranscriptForInspection(transcript);
                }

                return (
                  <SortableTrackLane
                    visibleRange={viewModel.waveformRequestRange}
                    aiProcessingState={aiProcessingState}
                    clipDrafts={clipsById}
                    detail={detail}
                    key={track.id}
                    lane={lane}
                    mutedTrackIds={mutedTrackIds}
                    playbackAssets={playbackAssets}
                    selectedClipIds={viewModel.selectedClipIds}
                    selectedLane={viewModel.selectedLaneIds.has(track.id)}
                    selectedRedactionIds={viewModel.selectedRedactionIds}
                    selectedVisualFrameIds={viewModel.selectedVisualFrameIds}
                    thumbnailStatus={thumbnailStatus}
                    timelineWidthPx={viewModel.timelineWidthPx}
                    timeRanges={viewModel.rangesByLaneId.get(track.id) ?? []}
                    totalDurationMs={viewModel.totalDurationMs}
                    track={track}
                    waveformTiles={waveformTiles}
                    visualFrames={visualFrames}
                    onInspectTranscript={handleSelectTranscript}
                    onSelectClip={handleSelectClip}
                    onSelectLane={handleSelectLane}
                    onSelectRedaction={handleSelectRedaction}
                    onSelectVisualFrame={selectVisualFrameForInspection}
                    onToggleTrackMuted={onToggleTrackMuted}
                  />
                );
              })}
            </SortableContext>
          </DndContext>
        </div>
      </ScrollArea>
    </div>
  );
}
