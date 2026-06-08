import {
  type PointerEvent,
  type WheelEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
} from '@daydream/shared';
import { Minus, Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { formatDuration } from '../../library-format';
import { timelineDurationMs } from '../../library-timeline';
import { TrackLane } from './track-lane';
import {
  buildRulerTicks,
  clampScrollLeft,
  clampTimelineMs,
  clampVisibleWindowMs,
  DEFAULT_TRACK_VIEWPORT_WIDTH_PX,
  INITIAL_TRACK_WINDOW_MS,
  minVisibleWindowMs,
  type RulerTick,
  TRACK_FOLLOW_EDGE_PX,
  TRACK_LABEL_WIDTH_PX,
  TRACK_RULER_HEIGHT_PX,
  TRACK_ZOOM_STEP,
} from './track-timeline-math';
import type { RecordingPlaybackController } from './use-recording-playback-controller';
import { useWaveformTileQueue } from './use-waveform-tile-queue';

export function TrackTimeline({
  detail,
  mutedTrackIds,
  onToggleTrackMuted,
  playback,
  playbackAssets,
  thumbnailStatus,
}: {
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  onToggleTrackMuted: (trackId: string) => void;
  playback: RecordingPlaybackController;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
}) {
  const { actions, state } = playback;
  const totalDurationMs = Math.max(1, timelineDurationMs(detail.segments));
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);
  const dragStateRef = useRef<{ active: boolean; pointerId: number | null }>({
    active: false,
    pointerId: null,
  });
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
    (clampTimelineMs(state.snapshot.displayedTimelineMs, totalDurationMs) /
      totalDurationMs) *
      timelineWidthPx;
  const zoomPercent = Math.max(
    100,
    Math.round((totalDurationMs / visibleWindowMs) * 100),
  );
  const canZoomIn = visibleWindowMs > minVisibleWindowMs(totalDurationMs);
  const canZoomOut = visibleWindowMs < totalDurationMs;
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
  const { waveformStatus, waveformTiles } = useWaveformTileQueue({
    detail,
    requestRange: waveformRequestRange,
    timelineWidthPx,
    totalDurationMs,
  });

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
  }, [detail.id, totalDurationMs]);

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) {
      return;
    }

    function updateViewportWidth() {
      setViewportWidth(
        Math.max(1, viewport?.clientWidth ?? DEFAULT_TRACK_VIEWPORT_WIDTH_PX),
      );
    }

    updateViewportWidth();
    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', updateViewportWidth);
      return () => window.removeEventListener('resize', updateViewportWidth);
    }

    const observer = new ResizeObserver(updateViewportWidth);
    observer.observe(viewport);
    return () => observer.disconnect();
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
      if (state.snapshot.isPlaying) {
        setFollowPlayhead(false);
      }
    }

    scrollViewport.addEventListener('scroll', handleScroll, { passive: true });
    return () => scrollViewport.removeEventListener('scroll', handleScroll);
  }, [state.snapshot.isPlaying]);

  useEffect(() => {
    if (!followPlayhead || !state.snapshot.isPlaying) {
      return;
    }
    keepPlayheadVisible(playheadLeftPx);
  }, [
    followPlayhead,
    keepPlayheadVisible,
    state.snapshot.isPlaying,
    playheadLeftPx,
  ]);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => setFollowPlayhead(true));

    return () => window.cancelAnimationFrame(frame);
  }, [state.snapshot.seekRevision]);

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

  function commitPointerTimelineMs(clientX: number) {
    const nextTimelineMs = timelineMsFromClientX(clientX);
    if (nextTimelineMs === null) {
      return;
    }
    actions.commitTimelineInput(nextTimelineMs);
    setFollowPlayhead(true);
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
    if (nextTimelineMs === null) {
      return;
    }

    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    dragStateRef.current = { active: true, pointerId: event.pointerId };
    actions.handleTimelineInput(nextTimelineMs);
    setFollowPlayhead(true);
  }

  function handleTimelinePointerMove(event: PointerEvent<HTMLDivElement>) {
    if (
      !dragStateRef.current.active ||
      dragStateRef.current.pointerId !== event.pointerId
    ) {
      return;
    }

    const nextTimelineMs = timelineMsFromClientX(event.clientX);
    if (nextTimelineMs !== null) {
      actions.handleTimelineInput(nextTimelineMs);
    }
  }

  function handleTimelinePointerEnd(event: PointerEvent<HTMLDivElement>) {
    if (
      !dragStateRef.current.active ||
      dragStateRef.current.pointerId !== event.pointerId
    ) {
      return;
    }

    dragStateRef.current = { active: false, pointerId: null };
    event.currentTarget.releasePointerCapture(event.pointerId);
    commitPointerTimelineMs(event.clientX);
  }

  function handleTimelineWheel(event: WheelEvent<HTMLDivElement>) {
    if (!event.ctrlKey) {
      return;
    }

    event.preventDefault();
    const zoomIn = event.deltaY < 0;
    zoomVisibleWindow(
      zoomIn
        ? visibleWindowMs / TRACK_ZOOM_STEP
        : visibleWindowMs * TRACK_ZOOM_STEP,
      event.clientX,
    );
  }

  function zoomVisibleWindow(nextWindowMs: number, anchorClientX?: number) {
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
      ) ?? clampTimelineMs(state.snapshot.displayedTimelineMs, totalDurationMs);
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
  }

  return (
    <div className="bg-background flex h-full flex-col">
      <div className="flex h-9 shrink-0 items-center justify-between gap-3 border-b px-3">
        <div className="text-sm font-medium">Tracks</div>
        <div className="flex min-w-0 items-center gap-2">
          <div className="text-muted-foreground truncate text-xs">
            {waveformTiles?.error ??
              (waveformStatus === 'loading'
                ? 'Loading waveforms...'
                : 'Video and audio timeline')}
          </div>
          <div
            aria-label="Track zoom"
            className="flex shrink-0 items-center gap-1"
          >
            <Button
              aria-label="Zoom out tracks"
              disabled={!canZoomOut}
              size="icon-xs"
              variant="ghost"
              onClick={() =>
                zoomVisibleWindow(visibleWindowMs * TRACK_ZOOM_STEP)
              }
            >
              <Minus className="size-3" />
            </Button>
            <div
              aria-live="polite"
              className="text-muted-foreground w-12 text-center text-xs tabular-nums"
            >
              {zoomPercent}%
            </div>
            <Button
              aria-label="Zoom in tracks"
              disabled={!canZoomIn}
              size="icon-xs"
              variant="ghost"
              onClick={() =>
                zoomVisibleWindow(visibleWindowMs / TRACK_ZOOM_STEP)
              }
            >
              <Plus className="size-3" />
            </Button>
          </div>
        </div>
      </div>
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
            width: TRACK_LABEL_WIDTH_PX + timelineWidthPx,
          }}
          onPointerCancel={handleTimelinePointerEnd}
          onPointerDown={handleTimelinePointerDown}
          onPointerMove={handleTimelinePointerMove}
          onPointerUp={handleTimelinePointerEnd}
          onWheel={handleTimelineWheel}
        >
          <div
            aria-hidden="true"
            className="bg-primary pointer-events-none absolute top-0 bottom-0 z-30 w-px"
            style={{ left: playheadLeftPx }}
          />
          <div
            className="grid"
            style={{
              gridTemplateColumns: `${TRACK_LABEL_WIDTH_PX}px ${timelineWidthPx}px`,
            }}
          >
            <div
              className="bg-background sticky left-0 z-40 border-r border-b px-3"
              style={{ height: TRACK_RULER_HEIGHT_PX }}
            />
            <TimeRuler ticks={rulerTicks} timelineWidthPx={timelineWidthPx} />
            {detail.tracks.map((track) => (
              <TrackLane
                detail={detail}
                key={track.id}
                mutedTrackIds={mutedTrackIds}
                playbackAssets={playbackAssets}
                thumbnailStatus={thumbnailStatus}
                timelineWidthPx={timelineWidthPx}
                totalDurationMs={totalDurationMs}
                track={track}
                waveformTiles={waveformTiles}
                onToggleTrackMuted={onToggleTrackMuted}
              />
            ))}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}

function TimeRuler({
  ticks,
  timelineWidthPx,
}: {
  ticks: RulerTick[];
  timelineWidthPx: number;
}) {
  return (
    <div
      aria-label="Time ruler"
      className="bg-background relative sticky top-0 z-20 border-b"
      style={{ height: TRACK_RULER_HEIGHT_PX, width: timelineWidthPx }}
    >
      {ticks.map((tick) => (
        <div
          className="border-muted-foreground/40 absolute bottom-0 border-l"
          key={`${tick.timeMs}-${tick.major ? 'major' : 'minor'}`}
          style={{
            height: tick.major ? 22 : 10,
            left: tick.leftPx,
          }}
        >
          {tick.major ? (
            <div className="text-muted-foreground mt-0.5 ml-1 text-[0.625rem] tabular-nums">
              {formatDuration(tick.timeMs)}
            </div>
          ) : null}
        </div>
      ))}
    </div>
  );
}
