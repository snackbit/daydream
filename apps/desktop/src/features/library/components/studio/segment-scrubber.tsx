import {
  type KeyboardEvent,
  type PointerEvent,
  useMemo,
  useRef,
  useState,
} from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryPlaybackSegment,
  LibraryRecordingSegment,
} from '@daydream/shared';
import { cn } from '@/lib/utils';
import { formatDuration, formatTimestamp } from '../../library-format';
import { segmentForTimeline } from '../../library-timeline';

type TimelineRegion =
  | {
      endMs: number;
      kind: 'pause';
      segment: LibraryRecordingSegment;
      startMs: number;
    }
  | {
      endMs: number;
      kind: 'segment';
      segment: LibraryRecordingSegment;
      startMs: number;
    };

interface HoverPreview {
  label: string;
  leftPercent: number;
  thumbnailUrl: null | string;
  title: string;
}

export function SegmentScrubber({
  currentTimelineMs,
  onCommit,
  onScrubUpdate,
  playbackAssets,
  segments,
  totalDurationMs,
}: {
  currentTimelineMs: number;
  onCommit: (timelineMs: number) => void;
  onScrubUpdate: (timelineMs: number) => void;
  playbackAssets: LibraryPlaybackAssets | null;
  segments: LibraryRecordingSegment[];
  totalDurationMs: number;
}) {
  const scrubberRef = useRef<HTMLDivElement | null>(null);
  const [dragging, setDragging] = useState(false);
  const [hoverPreview, setHoverPreview] = useState<HoverPreview | null>(null);
  const regions = useMemo(() => buildTimelineRegions(segments), [segments]);
  const progressPercent = percentOf(currentTimelineMs, totalDurationMs);

  function timelineMsFromPointer(event: PointerEvent<HTMLDivElement>) {
    const rect = event.currentTarget.getBoundingClientRect();
    const width = rect.width || 1;
    const position = Math.max(0, Math.min(width, event.clientX - rect.left));

    return (position / width) * totalDurationMs;
  }

  function updateHoverPreview(timelineMs: number) {
    setHoverPreview(
      previewForTimeline(timelineMs, totalDurationMs, regions, playbackAssets),
    );
  }

  function previewTimeline(timelineMs: number) {
    const resolvedTimelineMs = resolvePlayableTimelineMs(segments, timelineMs);
    onScrubUpdate(resolvedTimelineMs);
    updateHoverPreview(timelineMs);
  }

  function commitTimeline(timelineMs: number) {
    const resolvedTimelineMs = resolvePlayableTimelineMs(segments, timelineMs);
    setDragging(false);
    setHoverPreview(null);
    onCommit(resolvedTimelineMs);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const step = event.shiftKey ? 30_000 : 5_000;
    let nextTimelineMs: number | null = null;
    if (event.key === 'ArrowLeft') {
      nextTimelineMs = currentTimelineMs - step;
    } else if (event.key === 'ArrowRight') {
      nextTimelineMs = currentTimelineMs + step;
    } else if (event.key === 'Home') {
      nextTimelineMs = 0;
    } else if (event.key === 'End') {
      nextTimelineMs = totalDurationMs;
    }
    if (nextTimelineMs === null) {
      return;
    }

    event.preventDefault();
    commitTimeline(clampTimelineMs(nextTimelineMs, totalDurationMs));
  }

  return (
    <div className="relative py-1">
      {hoverPreview ? (
        <div
          aria-live="polite"
          className="pointer-events-none absolute bottom-full z-10 mb-3 w-44 -translate-x-1/2 overflow-hidden rounded-md border border-white/15 bg-neutral-950 text-white shadow-xl"
          style={{ left: `${hoverPreview.leftPercent}%` }}
        >
          {hoverPreview.thumbnailUrl ? (
            <img
              alt=""
              className="aspect-video w-full object-cover"
              src={hoverPreview.thumbnailUrl}
            />
          ) : (
            <div className="flex aspect-video w-full items-center justify-center bg-white/10 text-xs">
              {hoverPreview.title}
            </div>
          )}
          <div className="px-2 py-1 text-xs">
            <div className="font-medium">{hoverPreview.title}</div>
            <div className="text-white/70">{hoverPreview.label}</div>
          </div>
        </div>
      ) : null}

      <div
        aria-label="Playback position"
        aria-valuemax={Math.round(totalDurationMs)}
        aria-valuemin={0}
        aria-valuenow={Math.round(currentTimelineMs)}
        aria-valuetext={`${formatDuration(currentTimelineMs)} of ${formatDuration(
          totalDurationMs,
        )}`}
        className="group relative h-7 cursor-pointer outline-none"
        ref={scrubberRef}
        role="slider"
        tabIndex={0}
        onBlur={() => {
          setDragging(false);
          setHoverPreview(null);
        }}
        onKeyDown={handleKeyDown}
        onPointerCancel={(event) =>
          commitTimeline(timelineMsFromPointer(event))
        }
        onPointerDown={(event) => {
          event.currentTarget.setPointerCapture?.(event.pointerId);
          setDragging(true);
          previewTimeline(timelineMsFromPointer(event));
        }}
        onPointerEnter={(event) =>
          updateHoverPreview(timelineMsFromPointer(event))
        }
        onPointerLeave={() => {
          if (!dragging) {
            setHoverPreview(null);
          }
        }}
        onPointerMove={(event) => {
          const nextTimelineMs = timelineMsFromPointer(event);
          if (dragging) {
            previewTimeline(nextTimelineMs);
          } else {
            updateHoverPreview(nextTimelineMs);
          }
        }}
        onPointerUp={(event) => commitTimeline(timelineMsFromPointer(event))}
      >
        <div className="absolute top-1/2 right-0 left-0 h-2 -translate-y-1/2 overflow-hidden rounded-full bg-white/15 transition-[height] group-hover:h-3">
          {regions.map((region) => (
            <div
              aria-label={timelineRegionLabel(region)}
              className={cn(
                'absolute top-0 h-full',
                region.kind === 'segment'
                  ? 'bg-white/30'
                  : 'bg-white/10 bg-[repeating-linear-gradient(135deg,rgba(255,255,255,0.18)_0,rgba(255,255,255,0.18)_3px,transparent_3px,transparent_7px)]',
              )}
              data-playback-region={region.kind}
              key={`${region.kind}-${region.segment.id}-${region.startMs}`}
              style={{
                left: `${percentOf(region.startMs, totalDurationMs)}%`,
                width: `${Math.max(
                  0,
                  percentOf(region.endMs - region.startMs, totalDurationMs),
                )}%`,
              }}
            />
          ))}
          <div
            className="absolute top-0 left-0 h-full bg-white"
            style={{ width: `${progressPercent}%` }}
          />
        </div>
        {segments.map((segment) => {
          const startPercent = percentOf(
            segment.playbackOffsetMs,
            totalDurationMs,
          );
          const endPercent = percentOf(
            segment.playbackOffsetMs + (segment.durationMs ?? 0),
            totalDurationMs,
          );

          return (
            <div key={segment.id}>
              <div
                aria-label={`Segment ${segment.segmentIndex} starts`}
                className="absolute top-1/2 h-4 w-px -translate-y-1/2 bg-white/80"
                data-segment-boundary="start"
                style={{ left: `${startPercent}%` }}
              />
              <div
                aria-label={`Segment ${segment.segmentIndex} ends`}
                className="absolute top-1/2 h-4 w-px -translate-y-1/2 bg-white/45"
                data-segment-boundary="end"
                style={{ left: `${endPercent}%` }}
              />
            </div>
          );
        })}
        <div
          className="absolute top-1/2 size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border border-black/30 bg-white shadow transition-transform group-hover:scale-110"
          style={{ left: `${progressPercent}%` }}
        />
      </div>
    </div>
  );
}

function buildTimelineRegions(
  segments: LibraryRecordingSegment[],
): TimelineRegion[] {
  return segments.flatMap((segment) => {
    const durationMs = segment.durationMs ?? 0;
    const segmentStartMs = segment.playbackOffsetMs;
    const segmentEndMs = segmentStartMs + durationMs;
    const regions: TimelineRegion[] = [
      {
        endMs: segmentEndMs,
        kind: 'segment',
        segment,
        startMs: segmentStartMs,
      },
    ];
    const pauseEndMs = segmentEndMs + Math.max(0, segment.pauseAfterMs);
    if (pauseEndMs > segmentEndMs) {
      regions.push({
        endMs: pauseEndMs,
        kind: 'pause',
        segment,
        startMs: segmentEndMs,
      });
    }

    return regions;
  });
}

function previewForTimeline(
  timelineMs: number,
  totalDurationMs: number,
  regions: TimelineRegion[],
  playbackAssets: LibraryPlaybackAssets | null,
): HoverPreview | null {
  const clampedTimelineMs = clampTimelineMs(timelineMs, totalDurationMs);
  const region = regionForTimeline(regions, clampedTimelineMs);
  if (!region) {
    return null;
  }
  const leftPercent = percentOf(clampedTimelineMs, totalDurationMs);
  if (region.kind === 'pause') {
    return {
      label: `${formatDuration(region.endMs - region.startMs)} gap, resumes at ${formatTimestamp(
        nextSegmentStart(region.segment, regions),
      )}`,
      leftPercent,
      thumbnailUrl: null,
      title: 'Paused',
    };
  }

  const offsetMs = clampedTimelineMs - region.segment.playbackOffsetMs;
  const playbackSegment = playbackAssets?.segments.find(
    (segment) => segment.segmentId === region.segment.id,
  );
  const thumbnail = nearestThumbnail(playbackSegment, offsetMs);

  return {
    label: `Segment ${region.segment.segmentIndex} - ${formatDuration(offsetMs)}`,
    leftPercent,
    thumbnailUrl: thumbnail?.url ?? null,
    title: formatDuration(clampedTimelineMs),
  };
}

function regionForTimeline(
  regions: TimelineRegion[],
  timelineMs: number,
): TimelineRegion | null {
  return (
    regions.find(
      (region) => timelineMs >= region.startMs && timelineMs <= region.endMs,
    ) ?? null
  );
}

function resolvePlayableTimelineMs(
  segments: LibraryRecordingSegment[],
  timelineMs: number,
): number {
  const target = segmentForTimeline(segments, timelineMs);
  if (!target) {
    return timelineMs;
  }
  const segment = segments[target.index];
  if (!segment) {
    return timelineMs;
  }

  return segment.playbackOffsetMs + target.offsetMs;
}

function nearestThumbnail(
  playbackSegment: LibraryPlaybackSegment | undefined,
  offsetMs: number,
) {
  return playbackSegment?.thumbnails.reduce<
    LibraryPlaybackSegment['thumbnails'][number] | undefined
  >((nearest, thumbnail) => {
    if (!nearest) {
      return thumbnail;
    }

    return Math.abs(thumbnail.offsetMs - offsetMs) <
      Math.abs(nearest.offsetMs - offsetMs)
      ? thumbnail
      : nearest;
  }, undefined);
}

function nextSegmentStart(
  segment: LibraryRecordingSegment,
  regions: TimelineRegion[],
): number | null {
  const nextRegion = regions.find(
    (region) =>
      region.kind === 'segment' &&
      region.segment.segmentIndex > segment.segmentIndex,
  );

  return nextRegion?.segment.startedAtMs ?? null;
}

function timelineRegionLabel(region: TimelineRegion): string {
  if (region.kind === 'pause') {
    return `Paused after segment ${region.segment.segmentIndex}`;
  }

  return `Segment ${region.segment.segmentIndex}`;
}

function clampTimelineMs(timelineMs: number, totalDurationMs: number): number {
  return Math.max(0, Math.min(totalDurationMs, timelineMs));
}

function percentOf(value: number, total: number): number {
  if (total <= 0) {
    return 0;
  }

  return Math.max(0, Math.min(100, (value / total) * 100));
}
