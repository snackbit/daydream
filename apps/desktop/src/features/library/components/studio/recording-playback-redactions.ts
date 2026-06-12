import type { LibraryRecordingDetail } from '@daydream/shared';

export const REDACTION_START_GUARD_MS = 75;

export function redactedTracksAtTimelineMs(
  detail: LibraryRecordingDetail,
  timelineMs: number,
  startGuardMs = 0,
): Set<string> {
  const redactedTrackIds = new Set<string>();
  for (const redaction of detail.redactions) {
    if (
      timelineMs >= redaction.startOffsetMs - startGuardMs &&
      timelineMs < redaction.endOffsetMs
    ) {
      for (const trackId of redaction.trackIds) {
        redactedTrackIds.add(trackId);
      }
    }
  }

  return redactedTrackIds;
}

export function nextRedactionBoundaryDelayMs(
  detail: LibraryRecordingDetail,
  timelineMs: number,
): number | null {
  const nextBoundaryMs = detail.redactions.reduce<number | null>(
    (next, redaction) => {
      const boundaries = [
        redaction.startOffsetMs - REDACTION_START_GUARD_MS,
        redaction.endOffsetMs,
      ];
      for (const boundaryMs of boundaries) {
        if (boundaryMs > timelineMs && (next === null || boundaryMs < next)) {
          next = boundaryMs;
        }
      }

      return next;
    },
    null,
  );

  return nextBoundaryMs === null
    ? null
    : Math.max(0, Math.ceil(nextBoundaryMs - timelineMs));
}

export function nextUnredactedVideoTimelineMsForRedaction(
  detail: LibraryRecordingDetail,
  timelineMs: number,
  videoTrackIds: string[],
  totalDurationMs: number,
): number | null {
  const videoTrackIdSet = new Set(videoTrackIds);
  if (videoTrackIdSet.size === 0) {
    return null;
  }

  let candidateMs = Math.max(0, timelineMs);
  let advanced = true;
  while (advanced) {
    advanced = false;
    for (const redaction of detail.redactions) {
      if (
        !redaction.trackIds.some((trackId) => videoTrackIdSet.has(trackId)) ||
        candidateMs < redaction.startOffsetMs - REDACTION_START_GUARD_MS ||
        candidateMs >= redaction.endOffsetMs
      ) {
        continue;
      }

      candidateMs = Math.max(candidateMs, redaction.endOffsetMs);
      advanced = true;
    }
  }

  return candidateMs < totalDurationMs ? candidateMs : null;
}

export function setsEqual(left: Set<string>, right: Set<string>): boolean {
  if (left.size !== right.size) {
    return false;
  }

  for (const value of left) {
    if (!right.has(value)) {
      return false;
    }
  }

  return true;
}
