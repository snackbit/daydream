import {
  useRef,
  type MouseEvent,
  type PointerEvent,
  type RefObject,
} from 'react';
import type { RecordingPlaybackController } from '../../playback/use-recording-playback-controller';
import {
  normalizeTimeRangeSelection,
  subtractTimeRangeSelection,
  type StudioSelection,
} from '../state/studio-timeline-store';
import {
  clampTimelineMs,
  intersectTimelineLaneIds,
  selectionDistancePx,
  TRACK_LABEL_WIDTH_PX,
  TRACK_SELECTION_DRAG_THRESHOLD_PX,
  type TimelineLaneLayout,
} from './track-timeline-math';

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
  actions: Pick<
    RecordingPlaybackController['actions'],
    'handleTimelineInput' | 'commitTimelineInput' | 'cancelTimelineInput'
  >;
  clearSelection: () => void;
  contentRef: RefObject<HTMLDivElement | null>;
  selection: StudioSelection;
  setFollowPlayhead: (followPlayhead: boolean) => void;
  setSelection: (selection: StudioSelection) => void;
  timelineWidthPx: number;
  totalDurationMs: number;
}) {
  const dragStateRef = useRef<TimelineDragState | null>(null);
  const suppressClickRef = useRef(false);
  function handleTimelineClickCapture(event: MouseEvent<HTMLDivElement>) {
    // Portal events follow the React tree, but belong to their own surface.
    if (
      !(event.target instanceof Node) ||
      !event.currentTarget.contains(event.target)
    ) {
      return;
    }
    if (suppressClickRef.current) {
      suppressClickRef.current = false;
      event.preventDefault();
      event.stopPropagation();
    }
  }
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
      !(event.target instanceof Node) ||
      !event.currentTarget.contains(event.target) ||
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
    if (event.type === 'pointercancel') {
      actions.cancelTimelineInput();
      return;
    }
    if (drag.selecting) {
      suppressClickRef.current = true;
      applyPointerRangeSelection(event, drag);
      actions.cancelTimelineInput();
      return;
    }

    const nextTimelineMs = timelineMsFromClientX(event.clientX);
    if (nextTimelineMs !== null) {
      const local = clientPointToLocal(event.clientX, event.clientY);
      // Pointer capture retargets pointerup to the timeline; resolve its lane
      // from coordinates rather than the captured event target.
      const laneAtPoint = local
        ? intersectTimelineLaneIds({
            startY: local.y,
            endY: local.y + 1,
            lanes: activeLaneLayouts,
          })[0]
        : undefined;
      const inside = drag.baseSelection.ranges.some(
        (range) =>
          nextTimelineMs >= range.startMs &&
          nextTimelineMs <= range.endMs &&
          (drag.source === 'ruler' ||
            (laneAtPoint !== undefined && range.laneIds.includes(laneAtPoint))),
      );
      if (drag.mode === 'replace' && !inside) {
        clearSelection();
      }
      suppressClickRef.current = drag.baseSelection.ranges.length > 0;
      actions.commitTimelineInput(nextTimelineMs);
      setFollowPlayhead(true);
      return;
    }

    actions.cancelTimelineInput();
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
    handleTimelineClickCapture,
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
