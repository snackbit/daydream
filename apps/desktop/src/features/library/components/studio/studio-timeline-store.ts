import { create } from 'zustand';
import { STUDIO_TIMELINE_COLORS } from './studio-timeline-colors';
import { buildTimelineDrafts } from './studio-timeline-drafts';
import { moveIdBefore } from './studio-timeline-lane-order';
import {
  normalizeSelection,
  normalizeTimeRangeSelection,
  pruneSelection,
  toggleId,
} from './studio-timeline-selection';
import {
  emptySelection,
  type StudioTimelineState,
} from './studio-timeline-store-types';

export {
  STUDIO_TIMELINE_COLORS,
  studioColorById,
  type StudioTimelineColor,
} from './studio-timeline-colors';
export { studioClipId } from './studio-timeline-drafts';
export {
  mergeTimeRangeSelections,
  normalizeTimeRangeSelection,
  studioRangeId,
  subtractTimeRangeSelection,
} from './studio-timeline-selection';
export type {
  StudioClipDraft,
  StudioLaneDraft,
  StudioSelection,
  StudioTimelineState,
  StudioTimeRangeSelection,
} from './studio-timeline-store-types';

export const useStudioTimelineStore = create<StudioTimelineState>(
  (set, get) => ({
    clipsById: {},
    initializedRecordingId: null,
    laneOrder: [],
    lanesById: {},
    selection: emptySelection,
    clearSelection() {
      set({ selection: emptySelection });
    },
    initializeFromDetail(detail) {
      const current = get();
      const sameRecording = current.initializedRecordingId === detail.id;
      const { clipsById, laneOrder, lanesById } = buildTimelineDrafts(
        detail,
        sameRecording ? current : null,
      );

      set({
        clipsById,
        initializedRecordingId: detail.id,
        laneOrder,
        lanesById,
        selection: sameRecording
          ? pruneSelection(
              current.selection,
              lanesById,
              clipsById,
              new Set(detail.redactions.map((redaction) => redaction.id)),
            )
          : emptySelection,
      });
    },
    moveLane(activeId, overId) {
      if (activeId === overId) {
        return;
      }

      set((state) => ({
        laneOrder: moveIdBefore(state.laneOrder, activeId, overId),
      }));
    },
    renameClip(clipId, title) {
      set((state) => {
        const clip = state.clipsById[clipId];
        if (!clip) {
          return {};
        }

        return {
          clipsById: {
            ...state.clipsById,
            [clipId]: { ...clip, title },
          },
        };
      });
    },
    renameLane(trackId, label) {
      set((state) => {
        const lane = state.lanesById[trackId];
        if (!lane) {
          return {};
        }

        return {
          lanesById: {
            ...state.lanesById,
            [trackId]: { ...lane, label },
          },
        };
      });
    },
    selectClip(clipId, additive = false) {
      set((state) => ({
        selection: additive
          ? {
              clipIds: toggleId(state.selection.clipIds, clipId),
              laneIds: [],
              redactionIds: [],
              ranges: [],
            }
          : { clipIds: [clipId], laneIds: [], redactionIds: [], ranges: [] },
      }));
    },
    selectLane(trackId, additive = false) {
      set((state) => ({
        selection: additive
          ? {
              clipIds: [],
              laneIds: toggleId(state.selection.laneIds, trackId),
              redactionIds: [],
              ranges: [],
            }
          : { clipIds: [], laneIds: [trackId], redactionIds: [], ranges: [] },
      }));
    },
    selectRedaction(redactionId, additive = false) {
      set((state) => ({
        selection: additive
          ? {
              clipIds: [],
              laneIds: [],
              redactionIds: toggleId(state.selection.redactionIds, redactionId),
              ranges: [],
            }
          : {
              clipIds: [],
              laneIds: [],
              redactionIds: [redactionId],
              ranges: [],
            },
      }));
    },
    setLaneColor(trackId, colorId) {
      set((state) => {
        const lane = state.lanesById[trackId];
        if (
          !lane ||
          !STUDIO_TIMELINE_COLORS.some((color) => color.id === colorId)
        ) {
          return {};
        }

        return {
          lanesById: {
            ...state.lanesById,
            [trackId]: { ...lane, colorId },
          },
        };
      });
    },
    setSelection(selection) {
      set({ selection: normalizeSelection(selection) });
    },
    setTimeRangeSelection(selection, additive = false) {
      const nextRange = normalizeTimeRangeSelection(selection);
      set((state) => ({
        selection: additive
          ? normalizeSelection({
              clipIds: [],
              laneIds: [],
              redactionIds: [],
              ranges: [...state.selection.ranges, nextRange],
            })
          : {
              clipIds: [],
              laneIds: [],
              redactionIds: [],
              ranges: [nextRange],
            },
      }));
    },
  }),
);
