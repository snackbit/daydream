import { expect, test } from 'vitest';
import {
  combinedProcessingStatus,
  groupTimelineProcessingRanges,
} from './timeline-processing-groups';
import type { TimelineProcessingRange } from './timeline-processing-ranges';

function range(
  id: string,
  startOffsetMs: number,
  endOffsetMs: number,
  patch: Partial<TimelineProcessingRange> = {},
): TimelineProcessingRange {
  return {
    id,
    startOffsetMs,
    endOffsetMs,
    operation: 'ocr',
    status: 'queued',
    progress: 0,
    ...patch,
  };
}
test('consecutive chunks and overlapping screen operations form one bar without mutating source work', () => {
  const input = [
    range('later', 1000, 2000),
    range('observations', 0, 2000, { operation: 'screen-observations' }),
    range('first', 0, 1000, { status: 'running' }),
  ];
  const before = structuredClone(input);
  expect(groupTimelineProcessingRanges(input)).toEqual([
    {
      id: 'first',
      startOffsetMs: 0,
      endOffsetMs: 2000,
      status: 'running',
      operations: {
        ocr: ['running', 'queued'],
        'screen-observations': ['queued'],
      },
    },
  ]);
  expect(input).toEqual(before);
});
test('a successful or redacted gap stays uncovered and duplicate statuses are collapsed', () => {
  expect(
    groupTimelineProcessingRanges([
      range('a', 0, 1000),
      range('duplicate', 0, 1000),
      range('b', 2000, 3000),
    ]),
  ).toEqual([
    {
      id: 'a',
      startOffsetMs: 0,
      endOffsetMs: 1000,
      status: 'queued',
      operations: { ocr: ['queued'] },
    },
    {
      id: 'b',
      startOffsetMs: 2000,
      endOffsetMs: 3000,
      status: 'queued',
      operations: { ocr: ['queued'] },
    },
  ]);
  expect(groupTimelineProcessingRanges([])).toEqual([]);
});
test('failed work remains represented when neighboring work is running', () => {
  const group = groupTimelineProcessingRanges([
    range('a', 0, 1000, { status: 'failed' }),
    range('b', 1000, 2000, { status: 'running' }),
  ])[0];
  expect(group?.status).toBe('running');
  expect(group?.operations.ocr).toEqual(['failed', 'running']);
  expect(combinedProcessingStatus(['paused', 'failed'])).toBe('paused');
  expect(combinedProcessingStatus([])).toBe('queued');
});
