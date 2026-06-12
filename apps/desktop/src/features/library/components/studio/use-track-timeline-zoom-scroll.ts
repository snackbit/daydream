import { useCallback, useEffect } from 'react';
import type { Dispatch, RefObject, SetStateAction } from 'react';
import {
  clampScrollLeft,
  clampTimelineMs,
  clampVisibleWindowMs,
  TRACK_LABEL_WIDTH_PX,
  TRACK_ZOOM_STEP,
} from './track-timeline-math';
import { horizontalWheelDeltaPx } from './track-timeline-parts';

export function useTrackTimelineZoomScroll({
  contentRef,
  displayedTimelineMs,
  isPlaying,
  markProgrammaticScroll,
  setFollowPlayhead,
  setScrollLeft,
  setVisibleWindowMs,
  timelineMsFromClientX,
  timelineViewportWidth,
  totalDurationMs,
  viewportRef,
  visibleWindowMs,
}: {
  contentRef: RefObject<HTMLDivElement | null>;
  displayedTimelineMs: number;
  isPlaying: boolean;
  markProgrammaticScroll: () => void;
  setFollowPlayhead: (follow: boolean) => void;
  setScrollLeft: (scrollLeft: number) => void;
  setVisibleWindowMs: Dispatch<SetStateAction<number>>;
  timelineMsFromClientX: (clientX: number) => number | null;
  timelineViewportWidth: number;
  totalDurationMs: number;
  viewportRef: RefObject<HTMLDivElement | null>;
  visibleWindowMs: number;
}) {
  const zoomVisibleWindow = useCallback(
    (nextWindowMs: number, anchorClientX?: number) => {
      const viewport = viewportRef.current;
      const nextVisibleWindowMs = clampVisibleWindowMs(
        nextWindowMs,
        totalDurationMs,
      );
      if (nextVisibleWindowMs === visibleWindowMs) {
        return;
      }

      const viewportRect = viewport?.getBoundingClientRect();
      const anchorX =
        anchorClientX && viewportRect
          ? anchorClientX - viewportRect.left
          : TRACK_LABEL_WIDTH_PX + timelineViewportWidth / 2;
      const anchorTimelineMs =
        timelineMsFromClientX(
          anchorClientX ??
            (viewportRect?.left ?? 0) +
              TRACK_LABEL_WIDTH_PX +
              timelineViewportWidth / 2,
        ) ?? clampTimelineMs(displayedTimelineMs, totalDurationMs);
      const nextTimelineWidthPx = Math.max(
        timelineViewportWidth,
        (totalDurationMs / nextVisibleWindowMs) * timelineViewportWidth,
      );

      setVisibleWindowMs(nextVisibleWindowMs);
      window.requestAnimationFrame(() => {
        if (!viewport) {
          return;
        }
        const nextScrollLeft =
          TRACK_LABEL_WIDTH_PX +
          (anchorTimelineMs / totalDurationMs) * nextTimelineWidthPx -
          anchorX;
        markProgrammaticScroll();
        viewport.scrollLeft = clampScrollLeft(nextScrollLeft, viewport);
        setScrollLeft(viewport.scrollLeft);
      });
    },
    [
      displayedTimelineMs,
      markProgrammaticScroll,
      setScrollLeft,
      setVisibleWindowMs,
      timelineMsFromClientX,
      timelineViewportWidth,
      totalDurationMs,
      viewportRef,
      visibleWindowMs,
    ],
  );

  const handleTimelineWheel = useCallback(
    (event: WheelEvent) => {
      if (event.ctrlKey) {
        event.preventDefault();
        const zoomIn = event.deltaY < 0;
        zoomVisibleWindow(
          zoomIn
            ? visibleWindowMs / TRACK_ZOOM_STEP
            : visibleWindowMs * TRACK_ZOOM_STEP,
          event.clientX,
        );
        return;
      }

      if (!event.shiftKey) {
        return;
      }

      event.preventDefault();
      const viewport = viewportRef.current;
      if (!viewport) {
        return;
      }
      const deltaPx = horizontalWheelDeltaPx(event);
      if (deltaPx === 0) {
        return;
      }
      const nextScrollLeft = clampScrollLeft(
        viewport.scrollLeft + deltaPx,
        viewport,
      );
      viewport.scrollLeft = nextScrollLeft;
      setScrollLeft(nextScrollLeft);
      if (isPlaying) {
        setFollowPlayhead(false);
      }
    },
    [
      isPlaying,
      setFollowPlayhead,
      setScrollLeft,
      viewportRef,
      visibleWindowMs,
      zoomVisibleWindow,
    ],
  );

  useEffect(() => {
    const content = contentRef.current;
    if (!content) {
      return;
    }

    content.addEventListener('wheel', handleTimelineWheel, { passive: false });
    return () => content.removeEventListener('wheel', handleTimelineWheel);
  }, [contentRef, handleTimelineWheel]);

  return { zoomVisibleWindow };
}
