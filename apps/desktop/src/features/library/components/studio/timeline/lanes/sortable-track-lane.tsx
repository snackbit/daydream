import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { TRACK_LABEL_WIDTH_PX } from '../interactions/track-timeline-math';
import { TrackLane, type TrackLaneProps } from './track-lane';

export interface SortableTrackLaneProps extends Omit<
  TrackLaneProps,
  'dragHandleProps'
> {
  track: TrackLaneProps['track'];
}

export function SortableTrackLane(props: SortableTrackLaneProps) {
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id: props.track.id });
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
      <TrackLane {...props} dragHandleProps={{ ...attributes, ...listeners }} />
    </div>
  );
}
