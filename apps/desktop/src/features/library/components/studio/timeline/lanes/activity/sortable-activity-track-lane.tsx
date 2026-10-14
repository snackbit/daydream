import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { TRACK_LABEL_WIDTH_PX } from '../../interactions/track-timeline-math';
import {
  ActivityTrackLane,
  type ActivityTrackLaneProps,
} from './activity-track-lane';

export interface SortableActivityTrackLaneProps extends Omit<
  ActivityTrackLaneProps,
  'dragHandleProps'
> {
  lane: ActivityTrackLaneProps['lane'];
}

export function SortableActivityTrackLane(
  props: SortableActivityTrackLaneProps,
) {
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id: props.lane.trackId });
  return (
    <div
      ref={setNodeRef}
      className="group/lane grid"
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
        gridTemplateColumns: `${TRACK_LABEL_WIDTH_PX}px ${props.timelineWidthPx}px`,
        width: TRACK_LABEL_WIDTH_PX + props.timelineWidthPx,
      }}
    >
      <ActivityTrackLane
        {...props}
        dragHandleProps={{ ...attributes, ...listeners }}
      />
    </div>
  );
}
