import type {
  StudioClipDraft,
  StudioLaneDraft,
  StudioSelection,
  StudioTimeRangeSelection,
} from './studio-timeline-store-types';

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

export function normalizeSelection(
  selection: StudioSelection,
): StudioSelection {
  if (selection.ranges.length > 0) {
    return {
      clipIds: [],
      laneIds: [],
      redactionIds: [],
      ranges: mergeTimeRangeSelections(selection.ranges),
      visualFrameIds: [],
    };
  }

  if (selection.clipIds.length > 0) {
    return {
      clipIds: [...selection.clipIds],
      laneIds: [],
      redactionIds: [],
      ranges: [],
      visualFrameIds: [],
    };
  }

  if (selection.redactionIds.length > 0) {
    return {
      clipIds: [],
      laneIds: [],
      redactionIds: [...selection.redactionIds],
      ranges: [],
      visualFrameIds: [],
    };
  }

  if (selection.visualFrameIds.length > 0) {
    return {
      clipIds: [],
      laneIds: [],
      redactionIds: [],
      ranges: [],
      visualFrameIds: [...selection.visualFrameIds],
    };
  }

  return {
    clipIds: [],
    laneIds: [...selection.laneIds],
    redactionIds: [],
    ranges: [],
    visualFrameIds: [],
  };
}

export function pruneSelection(
  selection: StudioSelection,
  lanesById: Record<string, StudioLaneDraft>,
  clipsById: Record<string, StudioClipDraft>,
  redactionIds: Set<string>,
): StudioSelection {
  const next = normalizeSelection({
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
    visualFrameIds: [...selection.visualFrameIds],
  });
  return JSON.stringify(next) === JSON.stringify(selection) ? selection : next;
}

export function toggleId(ids: string[], id: string): string[] {
  return ids.includes(id)
    ? ids.filter((currentId) => currentId !== id)
    : [...ids, id];
}

function normalizeLaneIds(laneIds: string[]): string[] {
  return [...new Set(laneIds)].sort();
}

function laneSetKey(laneIds: string[]): string {
  return normalizeLaneIds(laneIds).join('\u0000');
}
