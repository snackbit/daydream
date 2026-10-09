import type { LibraryRecordingDetail } from '@daydream/shared';
import { STUDIO_ACTIVITY_LANES } from './studio-activity-lanes';
import { STUDIO_TIMELINE_COLORS } from './studio-timeline-colors';
import type {
  ExistingTimelineDraftState,
  StudioClipDraft,
  StudioLaneDraft,
} from './studio-timeline-store-types';

export function studioClipId(trackId: string, segmentId: string): string {
  return `${trackId}:${segmentId}`;
}

export function buildTimelineDrafts(
  detail: LibraryRecordingDetail,
  current: ExistingTimelineDraftState | null,
): {
  clipsById: Record<string, StudioClipDraft>;
  laneOrder: string[];
  lanesById: Record<string, StudioLaneDraft>;
} {
  const lanesById: Record<string, StudioLaneDraft> = {};
  const clipsById: Record<string, StudioClipDraft> = {};

  for (const [trackIndex, track] of detail.tracks.entries()) {
    const currentLane = current?.lanesById[track.id];
    lanesById[track.id] = {
      colorId:
        currentLane?.colorId ??
        STUDIO_TIMELINE_COLORS[trackIndex % STUDIO_TIMELINE_COLORS.length]!.id,
      description: track.codec ?? track.description,
      kind: track.kind,
      label: currentLane?.label ?? track.label,
      trackId: track.id,
    };

    for (const segment of detail.segments) {
      const clipId = studioClipId(track.id, segment.id);
      clipsById[clipId] = {
        segmentId: segment.id,
        title:
          current?.clipsById[clipId]?.title ??
          (segment.fragmentIds
            ? `Clip ${segment.segmentIndex}`
            : `Segment ${segment.segmentIndex}`),
        trackId: track.id,
      };
    }
  }
  for (const definition of STUDIO_ACTIVITY_LANES) {
    const currentLane = current?.lanesById[definition.id];
    lanesById[definition.id] = {
      colorId: currentLane?.colorId ?? definition.colorId,
      description: definition.description,
      kind: definition.kind,
      label: currentLane?.label ?? definition.label,
      trackId: definition.id,
    };
  }

  const trackIds = [
    ...detail.tracks.map((track) => track.id),
    ...STUDIO_ACTIVITY_LANES.map((lane) => lane.id),
  ];
  const trackIdSet = new Set(trackIds);
  const currentLaneOrderSet = new Set(current?.laneOrder ?? []);
  const laneOrder = current
    ? [
        ...current.laneOrder.filter((trackId) => trackIdSet.has(trackId)),
        ...trackIds.filter((trackId) => !currentLaneOrderSet.has(trackId)),
      ]
    : trackIds;

  return { clipsById, laneOrder, lanesById };
}
