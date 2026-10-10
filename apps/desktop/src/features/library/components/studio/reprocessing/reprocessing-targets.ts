import type { ReprocessingTarget } from '@daydream/shared';
import type { StudioTimeRangeSelection } from '../studio-timeline-store';

export function reprocessingTargets(
  ranges: StudioTimeRangeSelection[],
  mediaTrackIds: ReadonlySet<string>,
): ReprocessingTarget[] {
  const targets = new Map<string, ReprocessingTarget>();
  for (const range of ranges)
    for (const trackId of range.laneIds)
      if (mediaTrackIds.has(trackId)) {
        let target = targets.get(trackId);
        if (!target) {
          target = { trackId, ranges: [] };
          targets.set(trackId, target);
        }
        target.ranges?.push({
          startOffsetMs: Math.round(range.startMs),
          endOffsetMs: Math.round(range.endMs),
        });
      }
  return [...targets.values()];
}
