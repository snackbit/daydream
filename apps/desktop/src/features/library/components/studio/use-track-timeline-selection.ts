import { useRef, type PointerEvent, type RefObject } from 'react';
import {
  normalizeTimeRangeSelection,
  subtractTimeRangeSelection,
  type StudioSelection,
} from './studio-timeline-store';
import {
  clampTimelineMs,
  intersectTimelineLaneIds,
  selectionDistancePx,
  TRACK_LABEL_WIDTH_PX,
  TRACK_SELECTION_DRAG_THRESHOLD_PX,
  type TimelineLaneLayout,
} from './track-timeline-math';
import type { RecordingPlaybackController } from './use-recording-playback-controller';

type DragSource = 'lanes' | 'ruler';
type SelectionDragMode = 'add' | 'replace' | 'subtract';

interface TimelineDragState {
  baseSelection: StudioSelection;
  mode: SelectionDragMode;
  pointerId: number;
  selecting: boolean;
  source: DragSource;
  startClientX: number;
  startClientY: number;
  startedInLaneContent: boolean;
}

export function useTrackTimelineSelection({
  activeLaneIds,
  activeLaneLayouts,
  actions,
  contentRef,
  clearSelection,
  selection,
  setFollowPlayhead,
  setSelection,
  timelineWidthPx,
  totalDurationMs,
}: {
  activeLaneIds: string[];
  activeLaneLayouts: TimelineLaneLayout[];
  actions: RecordingPlaybackController['actions'];
  clearSelection: () => void;
  contentRef: RefObject<HTMLDivElement | null>;
  selection: StudioSelection;
  setFollowPlayhead: (followPlayhead: boolean) => void;
  setSelection: (selection: StudioSelection) => void;
  timelineWidthPx: number;
  totalDurationMs: number;
}) {
  const dragStateRef = useRef<TimelineDragState | null>(null);
  function timelineMsFromClientX(clientX: number): number | null {
    const content = contentRef.current;
    if (!content) {
      return null;
    }
    const x = clientX - content.getBoundingClientRect().left;
    const timelineX = x - TRACK_LABEL_WIDTH_PX;
    if (timelineX < 0) {
      return null;
    }

    return clampTimelineMs(
      (timelineX / timelineWidthPx) * totalDurationMs,
      totalDurationMs,
    );
  }

  function handleTimelinePointerDown(event: PointerEvent<HTMLDivElement>) {
    if (
      event.button !== 0 ||
      (event.target instanceof Element &&
        event.target.closest('[data-library-action]'))
    ) {
      return;
    }

    const nextTimelineMs = timelineMsFromClientX(event.clientX);
    if (
      nextTimelineMs === null ||
      !clientPointToLocal(event.clientX, event.clientY)
    ) {
      return;
    }

    event.preventDefault();
    try {
      event.currentTarget.setPointerCapture(event.pointerId);
    } catch {
      // Synthetic browser-test pointer events may not create an active pointer.
    }
    dragStateRef.current = {
      baseSelection: cloneSelection(selection),
      mode: selectionDragMode(event),
      pointerId: event.pointerId,
      selecting: false,
      source:
        event.target instanceof Element &&
        event.target.closest('[data-time-ruler]')
          ? 'ruler'
          : 'lanes',
      startClientX: event.clientX,
      startClientY: event.clientY,
      startedInLaneContent: isLaneContentTarget(event.target),
    };
    actions.handleTimelineInput(nextTimelineMs);
    setFollowPlayhead(true);
  }

  function handleTimelinePointerMove(event: PointerEvent<HTMLDivElement>) {
    const drag = dragStateRef.current;
    if (!drag || drag.pointerId !== event.pointerId) {
      return;
    }

    const nextTimelineMs = timelineMsFromClientX(event.clientX);
    if (nextTimelineMs === null) {
      return;
    }

    const distance = selectionDistancePx({
      currentX: event.clientX,
      currentY: event.clientY,
      startX: drag.startClientX,
      startY: drag.startClientY,
    });
    if (distance < TRACK_SELECTION_DRAG_THRESHOLD_PX) {
      actions.handleTimelineInput(nextTimelineMs);
      return;
    }

    drag.selecting = true;
    applyPointerRangeSelection(event, drag);
  }

  function handleTimelinePointerEnd(event: PointerEvent<HTMLDivElement>) {
    const drag = dragStateRef.current;
    if (!drag || drag.pointerId !== event.pointerId) {
      return;
    }

    dragStateRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    if (drag.selecting) {
      applyPointerRangeSelection(event, drag);
      return;
    }

    const nextTimelineMs = timelineMsFromClientX(event.clientX);
    if (nextTimelineMs !== null && !drag.startedInLaneContent) {
      clearSelection();
      actions.commitTimelineInput(nextTimelineMs);
      setFollowPlayhead(true);
    }
  }

  function applyPointerRangeSelection(
    event: PointerEvent<HTMLDivElement>,
    drag: TimelineDragState,
  ) {
    const startMs = timelineMsFromClientX(drag.startClientX);
    const endMs = timelineMsFromClientX(event.clientX);
    const currentPoint = clientPointToLocal(event.clientX, event.clientY);
    const startPoint = clientPointToLocal(drag.startClientX, drag.startClientY);
    if (startMs === null || endMs === null || !currentPoint || !startPoint) {
      return;
    }

    const laneIds =
      drag.source === 'ruler'
        ? activeLaneIds
        : intersectTimelineLaneIds({
            endY: currentPoint.y,
            lanes: activeLaneLayouts,
            startY: startPoint.y,
          });
    if (laneIds.length === 0) {
      return;
    }

    const nextRange = normalizeTimeRangeSelection({
      endMs,
      laneIds,
      startMs,
    });

    if (drag.mode === 'subtract') {
      setSelection({
        clipIds: [],
        laneIds: [],
        redactionIds: [],
        ranges: subtractTimeRangeSelection(
          drag.baseSelection.ranges,
          nextRange,
        ),
        visualFrameIds: [],
      });
      return;
    }

    setSelection(
      drag.mode === 'add'
        ? {
            clipIds: [],
            laneIds: [],
            redactionIds: [],
            ranges: [...drag.baseSelection.ranges, nextRange],
            visualFrameIds: [],
          }
        : {
            clipIds: [],
            laneIds: [],
            redactionIds: [],
            ranges: [nextRange],
            visualFrameIds: [],
          },
    );
  }

  function clientPointToLocal(clientX: number, clientY: number) {
    const content = contentRef.current;
    if (!content) {
      return null;
    }

    const rect = content.getBoundingClientRect();
    return {
      x: clientX - rect.left,
      y: clientY - rect.top,
    };
  }

  return {
    handleTimelinePointerDown,
    handleTimelinePointerEnd,
    handleTimelinePointerMove,
    timelineMsFromClientX,
  };
}

function selectionDragMode(
  event: PointerEvent<HTMLDivElement>,
): SelectionDragMode {
  const hasPrimaryModifier = event.ctrlKey || event.metaKey;
  if (hasPrimaryModifier && event.shiftKey) {
    return 'subtract';
  }

  if (hasPrimaryModifier || event.shiftKey) {
    return 'add';
  }

  return 'replace';
}

function cloneSelection(selection: StudioSelection): StudioSelection {
  return {
    clipIds: [...selection.clipIds],
    laneIds: [...selection.laneIds],
    redactionIds: [...selection.redactionIds],
    ranges: selection.ranges.map((range) => ({
      ...range,
      laneIds: [...range.laneIds],
    })),
    visualFrameIds: [...selection.visualFrameIds],
  };
}

function isLaneContentTarget(target: EventTarget): boolean {
  return (
    target instanceof Element && target.closest('[data-lane-content]') !== null
  );
}
