import type { LibraryRecordingDetail } from '@daydream/shared';
import { create } from 'zustand';
import { STUDIO_TIMELINE_COLORS } from './studio-timeline-colors';

export {
  STUDIO_TIMELINE_COLORS,
  studioColorById,
  type StudioTimelineColor,
} from './studio-timeline-colors';

export interface StudioLaneDraft {
  colorId: string;
  description: string;
  kind: 'desktop-audio' | 'microphone' | 'video';
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
}

interface StudioTimelineState {
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
  setLaneColor: (trackId: string, colorId: string) => void;
  setSelection: (selection: StudioSelection) => void;
  setTimeRangeSelection: (
    selection: Omit<StudioTimeRangeSelection, 'id'>,
    additive?: boolean,
  ) => void;
}

const emptySelection: StudioSelection = {
  clipIds: [],
  laneIds: [],
  redactionIds: [],
  ranges: [],
};

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

export function studioClipId(trackId: string, segmentId: string): string {
  return `${trackId}:${segmentId}`;
}

function buildTimelineDrafts(
  detail: LibraryRecordingDetail,
  current: StudioTimelineState | null,
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
          `Segment ${segment.segmentIndex}`,
        trackId: track.id,
      };
    }
  }

  const trackIds = detail.tracks.map((track) => track.id);
  const laneOrder = current
    ? [
        ...current.laneOrder.filter((trackId) => trackIds.includes(trackId)),
        ...trackIds.filter((trackId) => !current.laneOrder.includes(trackId)),
      ]
    : trackIds;

  return { clipsById, laneOrder, lanesById };
}

export function studioRangeId(
  laneIds: string[],
  startMs: number,
  endMs: number,
): string {
  return `${normalizeLaneIds(laneIds).join(',')}:${Math.round(
    startMs,
  )}-${Math.round(endMs)}`;
}

export function normalizeTimeRangeSelection({
  endMs,
  laneIds,
  startMs,
}: Omit<StudioTimeRangeSelection, 'id'>): StudioTimeRangeSelection {
  const normalizedStartMs = Math.min(startMs, endMs);
  const normalizedEndMs = Math.max(startMs, endMs);
  const normalizedLaneIds = normalizeLaneIds(laneIds);

  return {
    endMs: normalizedEndMs,
    id: studioRangeId(normalizedLaneIds, normalizedStartMs, normalizedEndMs),
    laneIds: normalizedLaneIds,
    startMs: normalizedStartMs,
  };
}

export function mergeTimeRangeSelections(
  ranges: StudioTimeRangeSelection[],
): StudioTimeRangeSelection[] {
  const normalizedRanges = ranges
    .map(normalizeTimeRangeSelection)
    .sort((left, right) => {
      const laneComparison = laneSetKey(left.laneIds).localeCompare(
        laneSetKey(right.laneIds),
      );
      if (laneComparison !== 0) {
        return laneComparison;
      }

      return left.startMs - right.startMs || left.endMs - right.endMs;
    });
  const merged: StudioTimeRangeSelection[] = [];

  for (const range of normalizedRanges) {
    const current = merged[merged.length - 1];
    if (
      current &&
      laneSetKey(current.laneIds) === laneSetKey(range.laneIds) &&
      range.startMs <= current.endMs
    ) {
      const endMs = Math.max(current.endMs, range.endMs);
      merged[merged.length - 1] = normalizeTimeRangeSelection({
        endMs,
        laneIds: current.laneIds,
        startMs: current.startMs,
      });
      continue;
    }

    merged.push(range);
  }

  return merged;
}

export function subtractTimeRangeSelection(
  ranges: StudioTimeRangeSelection[],
  cutter: StudioTimeRangeSelection,
): StudioTimeRangeSelection[] {
  const normalizedCutter = normalizeTimeRangeSelection(cutter);
  if (normalizedCutter.endMs <= normalizedCutter.startMs) {
    return mergeTimeRangeSelections(ranges);
  }

  const nextRanges: StudioTimeRangeSelection[] = [];
  for (const range of ranges.map(normalizeTimeRangeSelection)) {
    const affectedLaneIds = range.laneIds.filter((laneId) =>
      normalizedCutter.laneIds.includes(laneId),
    );
    const unaffectedLaneIds = range.laneIds.filter(
      (laneId) => !normalizedCutter.laneIds.includes(laneId),
    );
    const overlapsTime =
      normalizedCutter.startMs < range.endMs &&
      normalizedCutter.endMs > range.startMs;

    if (affectedLaneIds.length === 0 || !overlapsTime) {
      nextRanges.push(range);
      continue;
    }

    if (unaffectedLaneIds.length > 0) {
      nextRanges.push(
        normalizeTimeRangeSelection({
          endMs: range.endMs,
          laneIds: unaffectedLaneIds,
          startMs: range.startMs,
        }),
      );
    }

    if (normalizedCutter.startMs > range.startMs) {
      nextRanges.push(
        normalizeTimeRangeSelection({
          endMs: Math.min(normalizedCutter.startMs, range.endMs),
          laneIds: affectedLaneIds,
          startMs: range.startMs,
        }),
      );
    }

    if (normalizedCutter.endMs < range.endMs) {
      nextRanges.push(
        normalizeTimeRangeSelection({
          endMs: range.endMs,
          laneIds: affectedLaneIds,
          startMs: Math.max(normalizedCutter.endMs, range.startMs),
        }),
      );
    }
  }

  return mergeTimeRangeSelections(
    nextRanges.filter((range) => range.endMs > range.startMs),
  );
}

function normalizeSelection(selection: StudioSelection): StudioSelection {
  if (selection.ranges.length > 0) {
    return {
      clipIds: [],
      laneIds: [],
      redactionIds: [],
      ranges: mergeTimeRangeSelections(selection.ranges),
    };
  }

  if (selection.clipIds.length > 0) {
    return {
      clipIds: [...selection.clipIds],
      laneIds: [],
      redactionIds: [],
      ranges: [],
    };
  }

  if (selection.redactionIds.length > 0) {
    return {
      clipIds: [],
      laneIds: [],
      redactionIds: [...selection.redactionIds],
      ranges: [],
    };
  }

  return {
    clipIds: [],
    laneIds: [...selection.laneIds],
    redactionIds: [],
    ranges: [],
  };
}

function pruneSelection(
  selection: StudioSelection,
  lanesById: Record<string, StudioLaneDraft>,
  clipsById: Record<string, StudioClipDraft>,
  redactionIds: Set<string>,
): StudioSelection {
  return normalizeSelection({
    clipIds: selection.clipIds.filter((clipId) => clipId in clipsById),
    laneIds: selection.laneIds.filter((laneId) => laneId in lanesById),
    redactionIds: selection.redactionIds.filter((id) => redactionIds.has(id)),
    ranges: selection.ranges
      .map((range) =>
        normalizeTimeRangeSelection({
          endMs: range.endMs,
          laneIds: range.laneIds.filter((laneId) => laneId in lanesById),
          startMs: range.startMs,
        }),
      )
      .filter((range) => range.laneIds.length > 0),
  });
}

function normalizeLaneIds(laneIds: string[]): string[] {
  return [...new Set(laneIds)].sort();
}

function laneSetKey(laneIds: string[]): string {
  return normalizeLaneIds(laneIds).join('\u0000');
}

function toggleId(ids: string[], id: string): string[] {
  return ids.includes(id)
    ? ids.filter((currentId) => currentId !== id)
    : [...ids, id];
}

function moveIdBefore(
  ids: string[],
  activeId: string,
  overId: string,
): string[] {
  const activeIndex = ids.indexOf(activeId);
  const overIndex = ids.indexOf(overId);
  if (activeIndex === -1 || overIndex === -1) {
    return ids;
  }

  const next = [...ids];
  const [active] = next.splice(activeIndex, 1);
  next.splice(overIndex, 0, active!);
  return next;
}
