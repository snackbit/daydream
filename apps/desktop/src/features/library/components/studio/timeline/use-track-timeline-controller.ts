import {
  useCallback,
  useEffect,
  useState,
  type PointerEvent as ReactPointerEvent,
} from 'react';
import {
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import { sortableKeyboardCoordinates } from '@dnd-kit/sortable';
import { timelineDurationMs } from '../../../library-timeline';
import { useWaveformTileQueue } from '../media/waveforms/use-waveform-tile-queue';
import type { StudioInspectorSelection } from '../navigation/studio-inspector-selection';
import type { StudioVisualFrame } from '../observations/studio-visual-observations';
import { useSelectionRedactionMutation } from '../redactions/use-redaction-mutation';
import { reprocessingTargets } from '../reprocessing/reprocessing-targets';
import { useStudioResultWindowStore } from '../results/studio-result-window-store';
import {
  TRACK_LABEL_WIDTH_PX,
  TRACK_SELECTION_DRAG_THRESHOLD_PX,
  TRACK_ZOOM_STEP,
} from './interactions/track-timeline-math';
import { useTrackTimelineSelection } from './interactions/use-track-timeline-selection';
import {
  useTrackTimelinePlayheadFollow,
  useTrackTimelineViewport,
} from './interactions/use-track-timeline-viewport';
import { useTrackTimelineZoomScroll } from './interactions/use-track-timeline-zoom-scroll';
import { useStudioTimelineStore } from './state/studio-timeline-store';
import type { TrackTimelineProps } from './track-timeline';
import { useTrackTimelineViewModel } from './track-timeline-view-model';

export type TrackTimelineControllerInput = Pick<
  TrackTimelineProps,
  | 'readWaveformTiles'
  | 'detail'
  | 'onCreateRedactions'
  | 'onClearInspectorSelection'
  | 'onInspectActivity'
  | 'onInspectTranscript'
  | 'onUnredactRanges'
  | 'playback'
  | 'suspendFollow'
>;

export function useTrackTimelineController({
  readWaveformTiles,
  detail,
  onCreateRedactions,
  onClearInspectorSelection,
  onInspectActivity,
  onInspectTranscript,
  onUnredactRanges,
  playback,
  suspendFollow = false,
}: TrackTimelineControllerInput) {
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
    if (
      event.button !== 0 ||
      !(event.target instanceof Node) ||
      !event.currentTarget.contains(event.target)
    )
      return;
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

  return {
    actions,
    state,
    clipsById,
    lanesById,
    selection,
    redactionMutation,
    totalDurationMs,
    contentRef,
    followPlayhead,
    keepPlayheadVisible,
    viewportRef,
    visibleWindowMs,
    viewModel,
    hasTimeRangeSelection,
    targets,
    follow,
    menuIsOpen,
    menuOpen,
    waveformStatus,
    waveformTiles,
    sensors,
    handleDragEnd,
    handleTimelinePointerDown,
    handleTimelinePointerEnd,
    handleTimelinePointerMove,
    handleTimelineClickCapture,
    handleSelectClip,
    handleSelectLane,
    handleSelectRedaction,
    selectVisualFrameForInspection,
    handleInspectActivity,
    selectTranscriptForInspection,
    zoomSelection,
    zoomIn,
    zoomOut,
    zoomRange,
  };
}

export type TrackTimelineController = ReturnType<
  typeof useTrackTimelineController
>;
