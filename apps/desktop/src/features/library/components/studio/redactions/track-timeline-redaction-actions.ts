import type { CreateLibraryRedactionRangeInput } from '@daydream/shared';
import type { StudioTimeRangeSelection } from '../timeline/state/studio-timeline-store';

export function redactionRangesFromSelection(
  ranges: StudioTimeRangeSelection[],
): CreateLibraryRedactionRangeInput[] {
  return ranges.map((range) => ({
    endOffsetMs: Math.round(range.endMs),
    startOffsetMs: Math.round(range.startMs),
    trackIds: range.laneIds,
  }));
}
