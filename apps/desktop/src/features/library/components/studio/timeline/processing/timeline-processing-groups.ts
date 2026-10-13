import type {
  TimelineProcessingRange,
  TimelineProcessingStatus,
} from './timeline-processing-ranges';

export interface TimelineProcessingGroup {
  id: string;
  startOffsetMs: number;
  endOffsetMs: number;
  status: TimelineProcessingStatus;
  operations: Partial<
    Record<TimelineProcessingRange['operation'], TimelineProcessingStatus[]>
  >;
}

const priority: TimelineProcessingStatus[] = [
  'running',
  'queued',
  'planned',
  'paused',
  'failed',
  'cancelled',
];

export function combinedProcessingStatus(
  statuses: readonly TimelineProcessingStatus[],
): TimelineProcessingStatus {
  return priority.find((status) => statuses.includes(status)) ?? 'queued';
}

/** Group already eligible ranges; a real gap (including a redaction) stays a gap. */
export function groupTimelineProcessingRanges(
  ranges: readonly TimelineProcessingRange[],
): TimelineProcessingGroup[] {
  const groups: TimelineProcessingGroup[] = [];
  for (const range of ranges.toSorted(
    (a, b) =>
      a.startOffsetMs - b.startOffsetMs || a.endOffsetMs - b.endOffsetMs,
  )) {
    const last = groups.at(-1);
    if (last && range.startOffsetMs <= last.endOffsetMs) {
      last.endOffsetMs = Math.max(last.endOffsetMs, range.endOffsetMs);
      const statuses = last.operations[range.operation] ?? [];
      if (!statuses.includes(range.status)) statuses.push(range.status);
      last.operations[range.operation] = statuses;
      last.status = combinedProcessingStatus([last.status, range.status]);
    } else {
      groups.push({
        id: range.id,
        startOffsetMs: range.startOffsetMs,
        endOffsetMs: range.endOffsetMs,
        status: range.status,
        operations: { [range.operation]: [range.status] },
      });
    }
  }
  return groups;
}
