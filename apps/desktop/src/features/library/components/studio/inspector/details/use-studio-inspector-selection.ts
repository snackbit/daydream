import { useMemo } from 'react';
import type { LibraryRecordingRedaction } from '@daydream/shared';
import { useSelectedStudioFrames } from '../../observations/use-selected-studio-frames';
import {
  useStudioTimelineStore,
  type StudioLaneDraft,
} from '../../timeline/state/studio-timeline-store';
import { type StudioSidebarSegmentSelection } from '../editing/studio-sidebar-section-segment';
import type { StudioSidebarModel } from '../studio-sidebar';

export function useStudioInspectorSelection(model: StudioSidebarModel) {
  const clipsById = useStudioTimelineStore((state) => state.clipsById);
  const lanesById = useStudioTimelineStore((state) => state.lanesById);
  const timelineSelection = useStudioTimelineStore((state) => state.selection);
  const selectionIntent = useStudioTimelineStore(
    (state) => state.selectionIntent,
  );
  const clearSelection = useStudioTimelineStore(
    (state) => state.clearSelection,
  );

  const selectedLanes = timelineSelection.laneIds
    .map((laneId) => lanesById[laneId])
    .filter((lane): lane is StudioLaneDraft => Boolean(lane));
  const selectedSegments = timelineSelection.clipIds
    .map((clipId): StudioSidebarSegmentSelection | null => {
      const clip = clipsById[clipId];
      return clip ? { ...clip, clipId } : null;
    })
    .filter((segment): segment is StudioSidebarSegmentSelection =>
      Boolean(segment),
    );
  const selectedRedactions = timelineSelection.redactionIds
    .map((redactionId) =>
      model.detail.redactions.find((redaction) => redaction.id === redactionId),
    )
    .filter((redaction): redaction is LibraryRecordingRedaction =>
      Boolean(redaction),
    );
  const frameOwnership = useMemo(
    () =>
      JSON.stringify([
        model.detail.redactions,
        model.detail.startedAtMs,
        model.detail.segments.map((segment) => [
          segment.id,
          segment.playbackOffsetMs,
          segment.videoStreams?.map((stream) => [
            stream.trackId,
            stream.streamIndex,
          ]),
        ]),
      ]),
    [model.detail.redactions, model.detail.startedAtMs, model.detail.segments],
  );
  const selectedVisualFrames = useSelectedStudioFrames(
    model.detail.id,
    frameOwnership,
    timelineSelection.visualFrameIds,
    model.visualFrames,
  );
  const hasTimelineSelection =
    selectedLanes.length > 0 ||
    selectedSegments.length > 0 ||
    selectedRedactions.length > 0 ||
    selectedVisualFrames.length > 0 ||
    timelineSelection.ranges.length > 0;

  return {
    selectedLanes,
    selectedSegments,
    selectedRedactions,
    selectedVisualFrames,
    hasTimelineSelection,
    timelineSelection,
    selectionIntent,
    clearSelection,
  };
}
