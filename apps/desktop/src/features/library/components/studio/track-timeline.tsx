import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
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
import { useStudioTimelineStore } from './studio-timeline-store';
import {
  buildRulerTicks,
  clampScrollLeft,
  clampTimelineMs,
  clampVisibleWindowMs,
  DEFAULT_TRACK_VIEWPORT_WIDTH_PX,
  INITIAL_TRACK_WINDOW_MS,
  minVisibleWindowMs,
  TRACK_FOLLOW_EDGE_PX,
  TRACK_LABEL_WIDTH_PX,
  TRACK_RULER_HEIGHT_PX,
  TRACK_SELECTION_DRAG_THRESHOLD_PX,
  TRACK_ZOOM_STEP,
} from './track-timeline-math';
import {
  horizontalWheelDeltaPx,
  orderedDetailTracks,
  SortableTrackLane,
  TimelineRangeLabels,
  TimeRuler,
  TrackTimelineToolbar,
} from './track-timeline-parts';
import type { RecordingPlaybackController } from './use-recording-playback-controller';
import { useTrackTimelineSelection } from './use-track-timeline-selection';
import { useWaveformTileQueue } from './use-waveform-tile-queue';

export function TrackTimeline({
  aiProcessingState,
  detail,
  mutedTrackIds,
  onToggleTrackMuted,
  playback,
  playbackAssets,
  thumbnailStatus,
}: {
  aiProcessingState: RecordingAiProcessingState | null;
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  onToggleTrackMuted: (trackId: string) => void;
  playback: RecordingPlaybackController;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
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
  const selection = useStudioTimelineStore((store) => store.selection);
  const setSelection = useStudioTimelineStore((store) => store.setSelection);
  const totalDurationMs = Math.max(1, timelineDurationMs(detail.segments));
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
  const orderedTracks = useMemo(
    () => orderedDetailTracks(detail.tracks, laneOrder),
    [detail.tracks, laneOrder],
  );
  const activeLaneIds = orderedTracks.map((track) => track.id);
  const selectedClipIds = useMemo(
    () => new Set(selection.clipIds),
    [selection.clipIds],
  );
  const selectedLaneIds = useMemo(
    () => new Set(selection.laneIds),
    [selection.laneIds],
  );
  const hasTimeRangeSelection = selection.ranges.length > 0;
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
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: TRACK_SELECTION_DRAG_THRESHOLD_PX },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );
  const {
    handleTimelinePointerDown,
    handleTimelinePointerEnd,
    handleTimelinePointerMove,
    timelineMsFromClientX,
  } = useTrackTimelineSelection({
    actions,
    activeLaneIds,
    clearSelection,
    contentRef,
    selection,
    setFollowPlayhead,
    setSelection,
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

  function handleTimelineWheel(event: WheelEvent) {
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
    if (state.snapshot.isPlaying) {
      setFollowPlayhead(false);
    }
  }

  useEffect(() => {
    const content = contentRef.current;
    if (!content) {
      return;
    }

    content.addEventListener('wheel', handleTimelineWheel, { passive: false });
    return () => content.removeEventListener('wheel', handleTimelineWheel);
  });

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

  function handleDragEnd(event: DragEndEvent) {
    if (!event.over) {
      return;
    }
    moveLane(String(event.active.id), String(event.over.id));
  }

  return (
    <div className="bg-background flex h-full flex-col">
      <TrackTimelineToolbar
        canZoomIn={canZoomIn}
        canZoomOut={canZoomOut}
        hasTimeRangeSelection={hasTimeRangeSelection}
        statusText={
          waveformTiles?.error ??
          (waveformStatus === 'loading'
            ? 'Loading waveforms...'
            : 'Video and audio timeline')
        }
        zoomPercent={zoomPercent}
        onZoomIn={() => zoomVisibleWindow(visibleWindowMs / TRACK_ZOOM_STEP)}
        onZoomOut={() => zoomVisibleWindow(visibleWindowMs * TRACK_ZOOM_STEP)}
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
            width: TRACK_LABEL_WIDTH_PX + timelineWidthPx,
          }}
          onPointerCancel={handleTimelinePointerEnd}
          onPointerDown={handleTimelinePointerDown}
          onPointerMove={handleTimelinePointerMove}
          onPointerUp={handleTimelinePointerEnd}
        >
          <div
            aria-hidden="true"
            className="bg-primary pointer-events-none absolute top-0 bottom-0 z-30 w-px"
            style={{ left: playheadLeftPx }}
          />
          <TimelineRangeLabels
            ranges={selection.ranges}
            timelineWidthPx={timelineWidthPx}
            totalDurationMs={totalDurationMs}
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
          </div>
          <DndContext
            collisionDetection={closestCenter}
            sensors={sensors}
            onDragEnd={handleDragEnd}
          >
            <SortableContext
              items={activeLaneIds}
              strategy={verticalListSortingStrategy}
            >
              {orderedTracks.map((track) => {
                const lane = lanesById[track.id];
                if (!lane) {
                  return null;
                }

                return (
                  <SortableTrackLane
                    clipDrafts={clipsById}
                    detail={detail}
                    key={track.id}
                    lane={lane}
                    aiProcessingState={aiProcessingState}
                    mutedTrackIds={mutedTrackIds}
                    playbackAssets={playbackAssets}
                    selectedClipIds={selectedClipIds}
                    selectedLane={selectedLaneIds.has(track.id)}
                    thumbnailStatus={thumbnailStatus}
                    timelineWidthPx={timelineWidthPx}
                    timeRanges={selection.ranges.filter((range) =>
                      range.laneIds.includes(track.id),
                    )}
                    totalDurationMs={totalDurationMs}
                    track={track}
                    waveformTiles={waveformTiles}
                    onSelectClip={selectClip}
                    onSelectLane={selectLane}
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
