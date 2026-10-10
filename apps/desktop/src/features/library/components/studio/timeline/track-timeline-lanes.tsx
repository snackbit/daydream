import type { ComponentProps } from 'react';
import { isStudioActivityLaneKind } from '../studio-activity-lanes';
import type { StudioLaneDraft } from '../studio-timeline-store';
import {
  SortableActivityTrackLane,
  SortableTrackLane,
} from '../track-timeline-parts';
import type { useTrackTimelineViewModel } from '../track-timeline-view-model';

export interface TrackTimelineLanesProps {
  view: Pick<
    ReturnType<typeof useTrackTimelineViewModel>,
    'orderedLaneIds' | 'rangesByLaneId' | 'selectedLaneIds'
  >;
  lanesById: Record<string, StudioLaneDraft>;
  media: Omit<
    ComponentProps<typeof SortableTrackLane>,
    'track' | 'lane' | 'selectedLane' | 'timeRanges'
  >;
  activity: Omit<
    ComponentProps<typeof SortableActivityTrackLane>,
    'lane' | 'selectedLane' | 'timeRanges'
  >;
}
export function TrackTimelineLanes({
  view,
  lanesById,
  media,
  activity,
}: TrackTimelineLanesProps) {
  const tracks = new Map(media.detail.tracks.map((track) => [track.id, track]));
  return view.orderedLaneIds.map((id) => {
    const lane = lanesById[id];
    if (!lane) return null;
    const selectedLane = view.selectedLaneIds.has(id),
      timeRanges = view.rangesByLaneId.get(id) ?? [];
    if (isStudioActivityLaneKind(lane.kind))
      return (
        <SortableActivityTrackLane
          key={id}
          {...activity}
          lane={lane}
          selectedLane={selectedLane}
          timeRanges={timeRanges}
        />
      );
    const track = tracks.get(lane.trackId);
    return track ? (
      <SortableTrackLane
        key={id}
        {...media}
        track={track}
        lane={lane}
        selectedLane={selectedLane}
        timeRanges={timeRanges}
      />
    ) : null;
  });
}
