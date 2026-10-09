import { useCallback, useEffect, useRef, useState } from 'react';
import {
  clampScrollLeft,
  clampVisibleWindowMs,
  DEFAULT_TRACK_VIEWPORT_WIDTH_PX,
  INITIAL_TRACK_WINDOW_MS,
  TRACK_FOLLOW_EDGE_PX,
  TRACK_LABEL_WIDTH_PX,
} from './track-timeline-math';

export function useTrackTimelineViewport({
  detailId,
  isPlaying,
  totalDurationMs,
}: {
  detailId: string;
  isPlaying: boolean;
  totalDurationMs: number;
}) {
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);
  const programmaticScrollRef = useRef<number | null>(null);
  const [followPlayhead, setFollowPlayhead] = useState(true);
  const [scrollLeft, setScrollLeft] = useState(0);
  const [viewportWidth, setViewportWidth] = useState(
    DEFAULT_TRACK_VIEWPORT_WIDTH_PX,
  );
  const [visibleWindowMs, setVisibleWindowMs] = useState(() =>
    clampVisibleWindowMs(
      Math.min(INITIAL_TRACK_WINDOW_MS, totalDurationMs),
      totalDurationMs,
    ),
  );

  const markProgrammaticScroll = useCallback(() => {
    if (programmaticScrollRef.current !== null) {
      window.clearTimeout(programmaticScrollRef.current);
    }
    programmaticScrollRef.current = window.setTimeout(() => {
      programmaticScrollRef.current = null;
    }, 80);
  }, []);

  const keepPlayheadVisible = useCallback(
    (nextPlayheadLeftPx: number) => {
      const viewport = viewportRef.current;
      if (!viewport) {
        return;
      }

      const visibleStartPx = viewport.scrollLeft + TRACK_LABEL_WIDTH_PX;
      const visibleEndPx = viewport.scrollLeft + viewport.clientWidth;
      let nextScrollLeft = viewport.scrollLeft;
      if (nextPlayheadLeftPx > visibleEndPx - TRACK_FOLLOW_EDGE_PX) {
        nextScrollLeft =
          nextPlayheadLeftPx - viewport.clientWidth + TRACK_FOLLOW_EDGE_PX;
      } else if (nextPlayheadLeftPx < visibleStartPx + TRACK_FOLLOW_EDGE_PX) {
        nextScrollLeft =
          nextPlayheadLeftPx - TRACK_LABEL_WIDTH_PX - TRACK_FOLLOW_EDGE_PX;
      } else {
        return;
      }

      markProgrammaticScroll();
      viewport.scrollLeft = clampScrollLeft(nextScrollLeft, viewport);
      setScrollLeft(viewport.scrollLeft);
    },
    [markProgrammaticScroll],
  );

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => {
      setVisibleWindowMs((current) =>
        clampVisibleWindowMs(
          Math.min(current, INITIAL_TRACK_WINDOW_MS, totalDurationMs),
          totalDurationMs,
        ),
      );
      setScrollLeft(0);
      if (viewportRef.current) {
        viewportRef.current.scrollLeft = 0;
      }
    });

    return () => window.cancelAnimationFrame(frame);
  }, [detailId, totalDurationMs]);

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) {
      return;
    }

    let frame: number | null = null;
    let measuredWidth = DEFAULT_TRACK_VIEWPORT_WIDTH_PX;
    function updateViewportWidth() {
      if (frame !== null) return;
      frame = window.requestAnimationFrame(() => {
        frame = null;
        const nextWidth = Math.max(
          1,
          viewport?.clientWidth ?? DEFAULT_TRACK_VIEWPORT_WIDTH_PX,
        );
        if (nextWidth === measuredWidth) return;
        measuredWidth = nextWidth;
        setViewportWidth(nextWidth);
      });
    }

    updateViewportWidth();
    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', updateViewportWidth);
      return () => {
        window.removeEventListener('resize', updateViewportWidth);
        if (frame !== null) window.cancelAnimationFrame(frame);
      };
    }

    const observer = new ResizeObserver(updateViewportWidth);
    observer.observe(viewport);
    return () => {
      observer.disconnect();
      if (frame !== null) window.cancelAnimationFrame(frame);
    };
  }, []);

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) {
      return;
    }

    const scrollViewport = viewport;
    function handleScroll() {
      setScrollLeft(scrollViewport.scrollLeft);
      if (programmaticScrollRef.current !== null) {
        window.clearTimeout(programmaticScrollRef.current);
        programmaticScrollRef.current = window.setTimeout(() => {
          programmaticScrollRef.current = null;
        }, 80);
        return;
      }
      if (isPlaying) {
        setFollowPlayhead(false);
      }
    }

    scrollViewport.addEventListener('scroll', handleScroll, { passive: true });
    return () => scrollViewport.removeEventListener('scroll', handleScroll);
  }, [isPlaying]);

  return {
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
  };
}

export function useTrackTimelinePlayheadFollow({
  followPlayhead,
  isPlaying,
  keepPlayheadVisible,
  playheadLeftPx,
  seekRevision,
  setFollowPlayhead,
}: {
  followPlayhead: boolean;
  isPlaying: boolean;
  keepPlayheadVisible: (playheadLeftPx: number) => void;
  playheadLeftPx: number;
  seekRevision: number;
  setFollowPlayhead: (follow: boolean) => void;
}) {
  useEffect(() => {
    if (!followPlayhead || !isPlaying) {
      return;
    }
    keepPlayheadVisible(playheadLeftPx);
  }, [followPlayhead, keepPlayheadVisible, isPlaying, playheadLeftPx]);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => setFollowPlayhead(true));

    return () => window.cancelAnimationFrame(frame);
  }, [seekRevision, setFollowPlayhead]);
}
