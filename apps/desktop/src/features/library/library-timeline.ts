import type { LibraryRecordingSegment } from '@daydream/shared';

export function timelineDurationMs(
  segments: LibraryRecordingSegment[],
): number {
  return segments.reduce(
    (total, segment) =>
      Math.max(
        total,
        segment.playbackOffsetMs +
          (segment.durationMs ?? 0) +
          Math.max(0, segment.pauseAfterMs),
      ),
    0,
  );
}

export function segmentForTimeline(
  segments: LibraryRecordingSegment[],
  timelineMs: number,
): { index: number; offsetMs: number } | null {
  for (const [index, segment] of segments.entries()) {
    const durationMs = segment.durationMs ?? 0;
    const segmentEnd = segment.playbackOffsetMs + durationMs;
    if (timelineMs < segmentEnd) {
      return {
        index,
        offsetMs: Math.max(0, timelineMs - segment.playbackOffsetMs),
      };
    }
    if (
      index + 1 < segments.length &&
      timelineMs <= segmentEnd + segment.pauseAfterMs
    ) {
      return { index: Math.min(index + 1, segments.length - 1), offsetMs: 0 };
    }
  }

  const lastIndex = segments.length - 1;
  const last = segments[lastIndex];
  return last ? { index: lastIndex, offsetMs: last.durationMs ?? 0 } : null;
}
