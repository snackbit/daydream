import {
  type KeyboardEvent,
  type PointerEvent,
  useMemo,
  useRef,
  useState,
} from 'react';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingSegment,
  LibraryRecordingDetail,
} from '@daydream/shared';
import { cn } from '@/lib/utils';
import { formatDuration } from '../../library-format';
import { PlayerSeekbarRedactions } from './player-seekbar-redactions';
import {
  buildTimelineRegions,
  previewForTimeline,
  resolvePlayableTimelineMs,
  timelineRegionLabel,
  clampTimelineMs,
  percentOf,
  type HoverPreview,
} from './segment-scrubber-model';

export function SegmentScrubber({
  currentTimelineMs,
  onCommit,
  onScrubUpdate,
  playbackAssets,
  segments,
  totalDurationMs,
  privacy,
}: {
  currentTimelineMs: number;
  onCommit: (timelineMs: number) => void;
  onScrubUpdate: (timelineMs: number) => void;
  playbackAssets: LibraryPlaybackAssets | null;
  segments: LibraryRecordingSegment[];
  totalDurationMs: number;
  privacy: Pick<LibraryRecordingDetail, 'redactions' | 'tracks'>;
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
    if (
      privacy.redactions.some(
        (redaction) =>
          timelineMs >= redaction.startOffsetMs &&
          timelineMs < redaction.endOffsetMs &&
          privacy.tracks.some(
            (track) =>
              track.kind === 'video' && redaction.trackIds.includes(track.id),
          ),
      )
    ) {
      setHoverPreview({
        label: formatDuration(timelineMs),
        leftPercent: percentOf(timelineMs, totalDurationMs),
        thumbnailUrl: null,
        title: 'Redacted',
      });
      return;
    }
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
          role="tooltip"
          aria-live="polite"
          className="bg-popover text-popover-foreground pointer-events-none absolute bottom-full z-10 mb-3 w-44 -translate-x-1/2 overflow-hidden rounded-md border shadow-xl"
          style={{ left: `${hoverPreview.leftPercent}%` }}
        >
          {hoverPreview.thumbnailUrl ? (
            <img
              alt=""
              className="aspect-video w-full object-cover"
              src={hoverPreview.thumbnailUrl}
            />
          ) : (
            <div className="bg-muted text-muted-foreground flex aspect-video w-full items-center justify-center text-xs">
              {hoverPreview.title}
            </div>
          )}
          <div className="px-2 py-1 text-xs">
            <div className="font-medium">{hoverPreview.title}</div>
            <div className="text-muted-foreground">{hoverPreview.label}</div>
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
        className="group focus-visible:ring-ring relative h-7 cursor-pointer rounded-md outline-none focus-visible:ring-2"
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
        <div className="bg-muted-foreground/20 absolute top-1/2 right-0 left-0 h-2 -translate-y-1/2 overflow-hidden rounded-full transition-[height] group-hover:h-3">
          {regions.map((region) => (
            <div
              aria-label={timelineRegionLabel(region)}
              className={cn(
                'absolute top-0 h-full',
                region.kind === 'segment'
                  ? 'bg-muted-foreground'
                  : 'studio-redaction-pattern bg-muted',
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
            className="bg-primary absolute top-0 left-0 h-full"
            style={{ width: `${progressPercent}%` }}
          />
          <PlayerSeekbarRedactions
            detail={privacy}
            totalDurationMs={totalDurationMs}
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
                className="bg-muted-foreground absolute top-1/2 h-4 w-px -translate-y-1/2"
                data-segment-boundary="start"
                style={{ left: `${startPercent}%` }}
              />
              <div
                aria-label={`Segment ${segment.segmentIndex} ends`}
                className="bg-muted-foreground/60 absolute top-1/2 h-4 w-px -translate-y-1/2"
                data-segment-boundary="end"
                style={{ left: `${endPercent}%` }}
              />
            </div>
          );
        })}
        <div
          className="border-primary bg-background absolute top-1/2 size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 shadow-sm transition-transform group-hover:scale-110"
          style={{ left: `${progressPercent}%` }}
        />
      </div>
    </div>
  );
}
