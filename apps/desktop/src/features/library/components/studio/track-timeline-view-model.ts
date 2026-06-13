import { useMemo } from 'react';
import type { LibraryRecordingDetail } from '@daydream/shared';
import { timelineDurationMs } from '../../library-timeline';
import { redactionsIntersectingRanges } from './studio-redactions';
import type {
  StudioLaneDraft,
  StudioSelection,
  StudioTimeRangeSelection,
} from './studio-timeline-store';
import {
  buildRulerTicks,
  clampTimelineMs,
  DEFAULT_TRACK_VIEWPORT_WIDTH_PX,
  INITIAL_TRACK_WINDOW_MS,
  minVisibleWindowMs,
  TRACK_LABEL_WIDTH_PX,
  trackLaneHeightPxForKind,
} from './track-timeline-math';

export function useTrackTimelineViewModel({
  detail,
  displayedTimelineMs,
  laneOrder,
  lanesById,
  scrollLeft,
  selection,
  viewportWidth,
  visibleWindowMs,
}: {
  detail: LibraryRecordingDetail;
  displayedTimelineMs: number;
  laneOrder: string[];
  lanesById: Record<string, StudioLaneDraft>;
  scrollLeft: number;
  selection: StudioSelection;
  viewportWidth: number;
  visibleWindowMs: number;
}) {
  const totalDurationMs = Math.max(1, timelineDurationMs(detail.segments));
  const timelineViewportWidth = Math.max(
    1,
    viewportWidth - TRACK_LABEL_WIDTH_PX,
  );
  const timelineWidthPx = Math.max(
    timelineViewportWidth,
    (totalDurationMs / visibleWindowMs) * timelineViewportWidth,
  );
  const playheadLeftPx =
    TRACK_LABEL_WIDTH_PX +
    (clampTimelineMs(displayedTimelineMs, totalDurationMs) / totalDurationMs) *
      timelineWidthPx;
  const zoomPercent = Math.max(
    100,
    Math.round((totalDurationMs / visibleWindowMs) * 100),
  );
  const canZoomIn = visibleWindowMs > minVisibleWindowMs(totalDurationMs);
  const canZoomOut = visibleWindowMs < totalDurationMs;
  const orderedLaneIds = useMemo(
    () => laneOrder.filter((laneId) => Boolean(lanesById[laneId])),
    [laneOrder, lanesById],
  );
  const activeLaneIds = orderedLaneIds;
  const activeLaneLayouts = useMemo(
    () =>
      orderedLaneIds.map((laneId) => ({
        heightPx: trackLaneHeightPxForKind(lanesById[laneId]!.kind),
        id: laneId,
      })),
    [orderedLaneIds, lanesById],
  );
  const selectedClipIds = useMemo(
    () => new Set(selection.clipIds),
    [selection.clipIds],
  );
  const selectedLaneIds = useMemo(
    () => new Set(selection.laneIds),
    [selection.laneIds],
  );
  const selectedRedactionIds = useMemo(
    () => new Set(selection.redactionIds),
    [selection.redactionIds],
  );
  const selectedVisualFrameIds = useMemo(
    () => new Set(selection.visualFrameIds),
    [selection.visualFrameIds],
  );
  const selectedRangeRedactions = useMemo(
    () => redactionsIntersectingRanges(detail.redactions, selection.ranges),
    [detail.redactions, selection.ranges],
  );
  const rulerTicks = useMemo(
    () =>
      buildRulerTicks({
        scrollLeft,
        timelineViewportWidth,
        timelineWidthPx,
        totalDurationMs,
        visibleWindowMs,
      }),
    [
      scrollLeft,
      timelineViewportWidth,
      timelineWidthPx,
      totalDurationMs,
      visibleWindowMs,
    ],
  );
  const waveformRequestRange = useMemo(() => {
    const visibleStartMs = clampTimelineMs(
      (scrollLeft / timelineWidthPx) * totalDurationMs,
      totalDurationMs,
    );
    const visibleEndMs = clampTimelineMs(
      ((scrollLeft + timelineViewportWidth) / timelineWidthPx) *
        totalDurationMs,
      totalDurationMs,
    );
    const visibleRangeMs = Math.max(1, visibleEndMs - visibleStartMs);

    return {
      endTimelineMs: Math.min(totalDurationMs, visibleEndMs + visibleRangeMs),
      startTimelineMs: Math.max(0, visibleStartMs - visibleRangeMs),
    };
  }, [scrollLeft, timelineViewportWidth, timelineWidthPx, totalDurationMs]);
  const rangesByLaneId = useMemo(
    () => groupRangesByLaneId(selection.ranges),
    [selection.ranges],
  );

  return {
    activeLaneIds,
    activeLaneLayouts,
    canZoomIn,
    canZoomOut,
    rangesByLaneId,
    orderedLaneIds,
    playheadLeftPx,
    rulerTicks,
    selectedClipIds,
    selectedLaneIds,
    selectedRangeRedactions,
    selectedRedactionIds,
    selectedVisualFrameIds,
    timelineViewportWidth,
    timelineWidthPx,
    totalDurationMs,
    waveformRequestRange,
    zoomPercent,
  };
}

export function initialVisibleWindowMs(totalDurationMs: number): number {
  return Math.min(
    INITIAL_TRACK_WINDOW_MS,
    Math.max(1, totalDurationMs || DEFAULT_TRACK_VIEWPORT_WIDTH_PX),
  );
}

function groupRangesByLaneId(ranges: StudioTimeRangeSelection[]) {
  const grouped = new Map<string, StudioTimeRangeSelection[]>();
  for (const range of ranges) {
    for (const laneId of range.laneIds) {
      const laneRanges = grouped.get(laneId);
      if (laneRanges) {
        laneRanges.push(range);
      } else {
        grouped.set(laneId, [range]);
      }
    }
  }

  return grouped;
}
