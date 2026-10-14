import type { LibraryRecordingRedaction } from '@daydream/shared';
import type { StudioTimeRangeSelection } from '../timeline/state/studio-timeline-store';

export function redactionsIntersectingRanges(
  redactions: LibraryRecordingRedaction[],
  ranges: StudioTimeRangeSelection[],
): LibraryRecordingRedaction[] {
  if (ranges.length === 0 || redactions.length === 0) {
    return [];
  }

  return redactions.filter((redaction) =>
    ranges.some((range) => rangeIntersectsRedaction(range, redaction)),
  );
}

function rangeIntersectsRedaction(
  range: StudioTimeRangeSelection,
  redaction: LibraryRecordingRedaction,
): boolean {
  if (
    range.endMs <= redaction.startOffsetMs ||
    range.startMs >= redaction.endOffsetMs
  ) {
    return false;
  }

  return range.laneIds.some((laneId) => redaction.trackIds.includes(laneId));
}
