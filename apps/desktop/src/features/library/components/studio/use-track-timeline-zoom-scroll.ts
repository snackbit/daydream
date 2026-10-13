import { useCallback, useEffect, useLayoutEffect, useRef } from 'react';
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
  scrollLeft,
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
  markProgrammaticScroll: (target: number) => void;
  scrollLeft: number;
  setFollowPlayhead: (follow: boolean) => void;
  setScrollLeft: (scrollLeft: number) => void;
  setVisibleWindowMs: Dispatch<SetStateAction<number>>;
  timelineMsFromClientX: (clientX: number) => number | null;
  timelineViewportWidth: number;
  totalDurationMs: number;
  viewportRef: RefObject<HTMLDivElement | null>;
  visibleWindowMs: number;
}) {
  const pendingScroll = useRef<number | null>(null);
  useLayoutEffect(() => {
    const target = pendingScroll.current;
    pendingScroll.current = null;
    const viewport = viewportRef.current;
    if (target === null || !viewport) return;
    markProgrammaticScroll(target);
    viewport.scrollLeft = target;
  }, [scrollLeft, visibleWindowMs, viewportRef, markProgrammaticScroll]);
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

      const viewportRect =
        anchorClientX === undefined
          ? undefined
          : viewport?.getBoundingClientRect();
      const anchorX =
        anchorClientX !== undefined && viewportRect
          ? anchorClientX - viewportRect.left
          : TRACK_LABEL_WIDTH_PX + timelineViewportWidth / 2;
      const anchorTimelineMs =
        anchorClientX === undefined
          ? clampTimelineMs(
              (scrollLeft / timelineViewportWidth + 0.5) * visibleWindowMs,
              totalDurationMs,
            )
          : (timelineMsFromClientX(anchorClientX) ??
            clampTimelineMs(displayedTimelineMs, totalDurationMs));
      const nextTimelineWidthPx = Math.max(
        timelineViewportWidth,
        (totalDurationMs / nextVisibleWindowMs) * timelineViewportWidth,
      );

      const nextScrollLeft = Math.max(
        0,
        Math.min(
          Math.floor(nextTimelineWidthPx - timelineViewportWidth),
          Math.round(
            TRACK_LABEL_WIDTH_PX +
              (anchorTimelineMs / totalDurationMs) * nextTimelineWidthPx -
              anchorX,
          ),
        ),
      );
      pendingScroll.current = nextScrollLeft;
      setVisibleWindowMs(nextVisibleWindowMs);
      setScrollLeft(nextScrollLeft);
    },
    [
      displayedTimelineMs,
      scrollLeft,
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

  const zoomRange = useCallback(
    (startMs: number, endMs: number) => {
      const from = Math.max(0, Math.min(startMs, endMs));
      const to = Math.min(totalDurationMs, Math.max(startMs, endMs));
      const padding = Math.max(1, to - from) * 0.05;
      const start = Math.max(0, from - padding);
      const end = Math.min(totalDurationMs, to + padding);
      const windowMs = clampVisibleWindowMs(end - start, totalDurationMs);
      setFollowPlayhead(false);
      const center = (start + end) / 2;
      const nextWidth = (totalDurationMs / windowMs) * timelineViewportWidth;
      const nextScrollLeft = Math.max(
        0,
        Math.min(
          Math.floor(nextWidth - timelineViewportWidth),
          Math.round((center / windowMs - 0.5) * timelineViewportWidth),
        ),
      );
      pendingScroll.current = nextScrollLeft;
      setVisibleWindowMs(windowMs);
      setScrollLeft(nextScrollLeft);
    },
    [
      totalDurationMs,
      setFollowPlayhead,
      setVisibleWindowMs,
      timelineViewportWidth,
      setScrollLeft,
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

  return { zoomVisibleWindow, zoomRange };
}
