import { expect, test } from 'vitest';
import {
  mergeTimeRangeSelections,
  normalizeTimeRangeSelection,
  subtractTimeRangeSelection,
} from './studio-timeline-selection';

test('timeline range merging extends overlapping same-lane selections', () => {
  expect(
    mergeTimeRangeSelections([
      {
        endMs: 2_000,
        id: 'first',
        laneIds: ['desktop-audio-1', 'video-0'],
        startMs: 1_000,
      },
      {
        endMs: 3_000,
        id: 'second',
        laneIds: ['video-0', 'desktop-audio-1'],
        startMs: 1_500,
      },
    ]),
  ).toEqual([
    {
      endMs: 3_000,
      id: 'desktop-audio-1,video-0:1000-3000',
      laneIds: ['desktop-audio-1', 'video-0'],
      startMs: 1_000,
    },
  ]);
});

test('timeline range merging joins touching same-lane selections', () => {
  expect(
    mergeTimeRangeSelections([
      {
        endMs: 2_000,
        id: 'first',
        laneIds: ['video-0'],
        startMs: 1_000,
      },
      {
        endMs: 3_000,
        id: 'second',
        laneIds: ['video-0'],
        startMs: 2_000,
      },
    ]),
  ).toHaveLength(1);
});

test('timeline range merging keeps overlapping different-lane selections separate', () => {
  expect(
    mergeTimeRangeSelections([
      {
        endMs: 2_000,
        id: 'first',
        laneIds: ['video-0'],
        startMs: 1_000,
      },
      {
        endMs: 2_500,
        id: 'second',
        laneIds: ['desktop-audio-1'],
        startMs: 1_500,
      },
    ]),
  ).toHaveLength(2);
});

test('timeline range subtraction trims an overlapped edge', () => {
  expect(
    subtractTimeRangeSelection(
      [
        {
          endMs: 5_000,
          id: 'range',
          laneIds: ['video-0'],
          startMs: 1_000,
        },
      ],
      {
        endMs: 3_000,
        id: 'cutter',
        laneIds: ['video-0'],
        startMs: 1_000,
      },
    ),
  ).toEqual([
    {
      endMs: 5_000,
      id: 'video-0:3000-5000',
      laneIds: ['video-0'],
      startMs: 3_000,
    },
  ]);
});

test('timeline range subtraction splits a middle overlap', () => {
  expect(
    subtractTimeRangeSelection(
      [
        {
          endMs: 5_000,
          id: 'range',
          laneIds: ['video-0'],
          startMs: 1_000,
        },
      ],
      {
        endMs: 3_000,
        id: 'cutter',
        laneIds: ['video-0'],
        startMs: 2_000,
      },
    ),
  ).toEqual([
    {
      endMs: 2_000,
      id: 'video-0:1000-2000',
      laneIds: ['video-0'],
      startMs: 1_000,
    },
    {
      endMs: 5_000,
      id: 'video-0:3000-5000',
      laneIds: ['video-0'],
      startMs: 3_000,
    },
  ]);
});

test('timeline range subtraction removes a fully covered range', () => {
  expect(
    subtractTimeRangeSelection(
      [
        {
          endMs: 5_000,
          id: 'range',
          laneIds: ['video-0'],
          startMs: 1_000,
        },
      ],
      {
        endMs: 6_000,
        id: 'cutter',
        laneIds: ['video-0'],
        startMs: 0,
      },
    ),
  ).toEqual([]);
});

test('timeline range subtraction only cuts intersected lanes', () => {
  expect(
    subtractTimeRangeSelection(
      [
        {
          endMs: 5_000,
          id: 'range',
          laneIds: ['desktop-audio-1', 'video-0'],
          startMs: 1_000,
        },
      ],
      {
        endMs: 3_000,
        id: 'cutter',
        laneIds: ['video-0'],
        startMs: 2_000,
      },
    ),
  ).toEqual([
    {
      endMs: 5_000,
      id: 'desktop-audio-1:1000-5000',
      laneIds: ['desktop-audio-1'],
      startMs: 1_000,
    },
    {
      endMs: 2_000,
      id: 'video-0:1000-2000',
      laneIds: ['video-0'],
      startMs: 1_000,
    },
    {
      endMs: 5_000,
      id: 'video-0:3000-5000',
      laneIds: ['video-0'],
      startMs: 3_000,
    },
  ]);
});

test('timeline range normalization dedupes and sorts lane ids', () => {
  expect(
    normalizeTimeRangeSelection({
      endMs: 1_000,
      laneIds: ['video-0', 'desktop-audio-1', 'video-0'],
      startMs: 2_000,
    }),
  ).toEqual({
    endMs: 2_000,
    id: 'desktop-audio-1,video-0:1000-2000',
    laneIds: ['desktop-audio-1', 'video-0'],
    startMs: 1_000,
  });
});
