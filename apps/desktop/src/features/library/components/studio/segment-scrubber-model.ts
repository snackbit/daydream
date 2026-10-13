import type {
  LibraryRecordingSegment,
  LibraryPlaybackAssets,
  LibraryPlaybackSegment,
} from '@daydream/shared';
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

export interface HoverPreview {
  label: string;
  leftPercent: number;
  thumbnailUrl: null | string;
  title: string;
}

export function buildTimelineRegions(
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

/** Draw continuous recorded spans; fragment boundaries are only hover lookup data. */
export function continuousTimelineRegions(
  regions: TimelineRegion[],
): TimelineRegion[] {
  const spans: TimelineRegion[] = [];
  for (const region of regions) {
    const previous = spans.at(-1);
    if (previous?.kind === region.kind && region.startMs <= previous.endMs) {
      previous.endMs = Math.max(previous.endMs, region.endMs);
    } else spans.push({ ...region });
  }
  return spans;
}

export function previewForTimeline(
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
    label: formatDuration(timelineMs),
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
      (region) => timelineMs >= region.startMs && timelineMs < region.endMs,
    ) ?? null
  );
}

export function resolvePlayableTimelineMs(
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

export function timelineRegionLabel(region: TimelineRegion): string {
  if (region.kind === 'pause') {
    return 'Recording paused';
  }

  return 'Recording';
}

export function clampTimelineMs(
  timelineMs: number,
  totalDurationMs: number,
): number {
  return Math.max(0, Math.min(totalDurationMs, timelineMs));
}

export function percentOf(value: number, total: number): number {
  if (total <= 0) {
    return 0;
  }

  return Math.max(0, Math.min(100, (value / total) * 100));
}
