import type { LibraryRecordingDetail } from '@daydream/shared';
import type { StudioActivityLaneKind } from './studio-activity-lanes';

export type StudioMediaLaneKind = 'desktop-audio' | 'microphone' | 'video';
export type StudioLaneKind = StudioActivityLaneKind | StudioMediaLaneKind;

export interface StudioLaneDraft {
  colorId: string;
  description: string;
  kind: StudioLaneKind;
  label: string;
  trackId: string;
}

export interface StudioClipDraft {
  segmentId: string;
  title: string;
  trackId: string;
}

export interface StudioTimeRangeSelection {
  endMs: number;
  id: string;
  laneIds: string[];
  startMs: number;
}

export interface StudioSelection {
  clipIds: string[];
  laneIds: string[];
  redactionIds: string[];
  ranges: StudioTimeRangeSelection[];
  visualFrameIds: string[];
}

export interface StudioTimelineState {
  clipsById: Record<string, StudioClipDraft>;
  initializedRecordingId: string | null;
  laneOrder: string[];
  lanesById: Record<string, StudioLaneDraft>;
  selection: StudioSelection;
  clearSelection: () => void;
  initializeFromDetail: (detail: LibraryRecordingDetail) => void;
  moveLane: (activeId: string, overId: string) => void;
  renameClip: (clipId: string, title: string) => void;
  renameLane: (trackId: string, label: string) => void;
  selectClip: (clipId: string, additive?: boolean) => void;
  selectLane: (trackId: string, additive?: boolean) => void;
  selectRedaction: (redactionId: string, additive?: boolean) => void;
  selectVisualFrame: (visualFrameId: string, additive?: boolean) => void;
  setLaneColor: (trackId: string, colorId: string) => void;
  setSelection: (selection: StudioSelection) => void;
  setTimeRangeSelection: (
    selection: Omit<StudioTimeRangeSelection, 'id'>,
    additive?: boolean,
  ) => void;
}

export type ExistingTimelineDraftState = Pick<
  StudioTimelineState,
  'clipsById' | 'laneOrder' | 'lanesById'
>;

export const emptySelection: StudioSelection = {
  clipIds: [],
  laneIds: [],
  redactionIds: [],
  ranges: [],
  visualFrameIds: [],
};
