import type { LibraryRecordingDetail } from '@daydream/shared';
import { create } from 'zustand';

export interface StudioTimelineColor {
  border: string;
  fill: string;
  id: string;
  label: string;
  strong: string;
}

export const STUDIO_TIMELINE_COLORS: StudioTimelineColor[] = [
  {
    border: '#2563eb',
    fill: '#dbeafe',
    id: 'blue',
    label: 'Blue',
    strong: '#1d4ed8',
  },
  {
    border: '#7c3aed',
    fill: '#ede9fe',
    id: 'violet',
    label: 'Violet',
    strong: '#6d28d9',
  },
  {
    border: '#0891b2',
    fill: '#cffafe',
    id: 'cyan',
    label: 'Cyan',
    strong: '#0e7490',
  },
  {
    border: '#059669',
    fill: '#d1fae5',
    id: 'emerald',
    label: 'Emerald',
    strong: '#047857',
  },
  {
    border: '#65a30d',
    fill: '#ecfccb',
    id: 'lime',
    label: 'Lime',
    strong: '#4d7c0f',
  },
  {
    border: '#ca8a04',
    fill: '#fef3c7',
    id: 'amber',
    label: 'Amber',
    strong: '#a16207',
  },
  {
    border: '#ea580c',
    fill: '#ffedd5',
    id: 'orange',
    label: 'Orange',
    strong: '#c2410c',
  },
  {
    border: '#dc2626',
    fill: '#fee2e2',
    id: 'red',
    label: 'Red',
    strong: '#b91c1c',
  },
  {
    border: '#db2777',
    fill: '#fce7f3',
    id: 'pink',
    label: 'Pink',
    strong: '#be185d',
  },
  {
    border: '#9333ea',
    fill: '#f3e8ff',
    id: 'purple',
    label: 'Purple',
    strong: '#7e22ce',
  },
  {
    border: '#0d9488',
    fill: '#ccfbf1',
    id: 'teal',
    label: 'Teal',
    strong: '#0f766e',
  },
  {
    border: '#0284c7',
    fill: '#e0f2fe',
    id: 'sky',
    label: 'Sky',
    strong: '#0369a1',
  },
  {
    border: '#4f46e5',
    fill: '#e0e7ff',
    id: 'indigo',
    label: 'Indigo',
    strong: '#4338ca',
  },
  {
    border: '#16a34a',
    fill: '#dcfce7',
    id: 'green',
    label: 'Green',
    strong: '#15803d',
  },
  {
    border: '#84cc16',
    fill: '#f7fee7',
    id: 'chartreuse',
    label: 'Chartreuse',
    strong: '#65a30d',
  },
  {
    border: '#d97706',
    fill: '#fef3c7',
    id: 'gold',
    label: 'Gold',
    strong: '#b45309',
  },
  {
    border: '#f97316',
    fill: '#ffedd5',
    id: 'tangerine',
    label: 'Tangerine',
    strong: '#ea580c',
  },
  {
    border: '#e11d48',
    fill: '#ffe4e6',
    id: 'rose',
    label: 'Rose',
    strong: '#be123c',
  },
  {
    border: '#c026d3',
    fill: '#fae8ff',
    id: 'fuchsia',
    label: 'Fuchsia',
    strong: '#a21caf',
  },
  {
    border: '#6d28d9',
    fill: '#ede9fe',
    id: 'grape',
    label: 'Grape',
    strong: '#5b21b6',
  },
  {
    border: '#475569',
    fill: '#f1f5f9',
    id: 'slate',
    label: 'Slate',
    strong: '#334155',
  },
  {
    border: '#78716c',
    fill: '#f5f5f4',
    id: 'stone',
    label: 'Stone',
    strong: '#57534e',
  },
  {
    border: '#0f766e',
    fill: '#ccfbf1',
    id: 'sea',
    label: 'Sea',
    strong: '#115e59',
  },
  {
    border: '#0369a1',
    fill: '#e0f2fe',
    id: 'steel',
    label: 'Steel',
    strong: '#075985',
  },
];

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
          ? pruneSelection(current.selection, lanesById, clipsById)
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
              ranges: [],
            }
          : { clipIds: [clipId], laneIds: [], ranges: [] },
      }));
    },
    selectLane(trackId, additive = false) {
      set((state) => ({
        selection: additive
          ? {
              clipIds: [],
              laneIds: toggleId(state.selection.laneIds, trackId),
              ranges: [],
            }
          : { clipIds: [], laneIds: [trackId], ranges: [] },
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
              ranges: [...state.selection.ranges, nextRange],
            })
          : {
              clipIds: [],
              laneIds: [],
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

export function studioColorById(colorId: string): StudioTimelineColor {
  return (
    STUDIO_TIMELINE_COLORS.find((color) => color.id === colorId) ??
    STUDIO_TIMELINE_COLORS[0]!
  );
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
      ranges: mergeTimeRangeSelections(selection.ranges),
    };
  }

  if (selection.clipIds.length > 0) {
    return {
      clipIds: [...selection.clipIds],
      laneIds: [],
      ranges: [],
    };
  }

  return {
    clipIds: [],
    laneIds: [...selection.laneIds],
    ranges: [],
  };
}

function pruneSelection(
  selection: StudioSelection,
  lanesById: Record<string, StudioLaneDraft>,
  clipsById: Record<string, StudioClipDraft>,
): StudioSelection {
  return normalizeSelection({
    clipIds: selection.clipIds.filter((clipId) => clipId in clipsById),
    laneIds: selection.laneIds.filter((laneId) => laneId in lanesById),
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
