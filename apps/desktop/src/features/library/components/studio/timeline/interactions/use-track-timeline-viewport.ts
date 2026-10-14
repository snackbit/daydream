import { useCallback, useEffect, useRef, useState } from 'react';
import {
  clampVisibleWindowMs,
  DEFAULT_TRACK_VIEWPORT_WIDTH_PX,
  INITIAL_TRACK_WINDOW_MS,
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

  const markProgrammaticScroll = useCallback((target: number) => {
    programmaticScrollRef.current = target;
  }, []);

  const keepPlayheadVisible = useCallback(
    (nextPlayheadLeftPx: number) => {
      const viewport = viewportRef.current;
      if (!viewport) {
        return;
      }

      const visibleStartPx = scrollLeft + TRACK_LABEL_WIDTH_PX;
      const visibleEndPx = scrollLeft + viewportWidth;
      const usableWidth = Math.max(1, viewportWidth - TRACK_LABEL_WIDTH_PX);
      const aheadPx = Math.max(1, usableWidth * 0.1);
      let nextScrollLeft = scrollLeft;
      if (nextPlayheadLeftPx > visibleEndPx - aheadPx) {
        nextScrollLeft =
          nextPlayheadLeftPx - TRACK_LABEL_WIDTH_PX - usableWidth * 0.5;
      } else if (nextPlayheadLeftPx < visibleStartPx) {
        nextScrollLeft = nextPlayheadLeftPx - TRACK_LABEL_WIDTH_PX - aheadPx;
      } else {
        return;
      }

      const width = Math.max(
        usableWidth,
        (totalDurationMs / visibleWindowMs) * usableWidth,
      );
      const target = Math.max(
        0,
        Math.min(Math.floor(width - usableWidth), Math.round(nextScrollLeft)),
      );
      if (target === scrollLeft) return;
      viewport.scrollLeft = target;
      programmaticScrollRef.current = target;
      setScrollLeft(target);
    },
    [scrollLeft, viewportWidth, totalDurationMs, visibleWindowMs],
  );

  const ownerRef = useRef(detailId);
  useEffect(() => {
    if (ownerRef.current === detailId) {
      setVisibleWindowMs((current) =>
        clampVisibleWindowMs(current, totalDurationMs),
      );
      return;
    }
    ownerRef.current = detailId;
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
    let previousHorizontal = scrollViewport.scrollLeft;
    function handleScroll() {
      if (scrollViewport.scrollLeft === previousHorizontal) return;
      previousHorizontal = scrollViewport.scrollLeft;
      setScrollLeft(scrollViewport.scrollLeft);
      if (programmaticScrollRef.current === scrollViewport.scrollLeft) {
        return;
      }
      programmaticScrollRef.current = null;
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
  readPlayheadLeftPx,
  seekRevision,
  setFollowPlayhead,
  suspended = false,
}: {
  followPlayhead: boolean;
  isPlaying: boolean;
  keepPlayheadVisible: (playheadLeftPx: number) => void;
  playheadLeftPx: number;
  readPlayheadLeftPx?: (() => number) | undefined;
  seekRevision: number;
  setFollowPlayhead: (follow: boolean) => void;
  suspended?: boolean;
}) {
  useEffect(() => {
    if (!followPlayhead || !isPlaying || suspended) {
      return;
    }
    keepPlayheadVisible(readPlayheadLeftPx?.() ?? playheadLeftPx);
  }, [
    followPlayhead,
    keepPlayheadVisible,
    isPlaying,
    playheadLeftPx,
    readPlayheadLeftPx,
    suspended,
  ]);

  const previousPlaying = useRef(isPlaying);
  const lastSeek = useRef(seekRevision);
  useEffect(() => {
    if (suspended) return undefined;
    if (
      lastSeek.current !== seekRevision ||
      (!previousPlaying.current && isPlaying)
    ) {
      const frame = window.requestAnimationFrame(() => {
        setFollowPlayhead(true);
        keepPlayheadVisible(readPlayheadLeftPx?.() ?? playheadLeftPx);
      });
      lastSeek.current = seekRevision;
      previousPlaying.current = isPlaying;
      return () => window.cancelAnimationFrame(frame);
    }
    previousPlaying.current = isPlaying;
    return undefined;
  }, [
    seekRevision,
    isPlaying,
    keepPlayheadVisible,
    playheadLeftPx,
    readPlayheadLeftPx,
    setFollowPlayhead,
    suspended,
  ]);
}
