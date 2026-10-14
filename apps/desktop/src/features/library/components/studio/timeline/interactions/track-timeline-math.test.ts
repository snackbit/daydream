import { expect, test } from 'vitest';
import {
  intersectLaneIds,
  intersectTimelineLaneIds,
  selectionDistancePx,
} from './track-timeline-math';

test('timeline math intersects only lanes touched by a vertical marquee', () => {
  expect(
    intersectLaneIds({
      endY: 170,
      laneIds: ['video-0', 'desktop-audio-1', 'microphone-1'],
      startY: 92,
    }),
  ).toEqual(['video-0', 'desktop-audio-1']);
});

test('timeline math intersects mixed-height timeline lanes', () => {
  expect(
    intersectTimelineLaneIds({
      endY: 248,
      lanes: [
        { heightPx: 96, id: 'video-0' },
        { heightPx: 96, id: 'desktop-audio-1' },
        { heightPx: 24, id: 'activity:mouse' },
        { heightPx: 24, id: 'activity:keyboard' },
      ],
      startY: 224,
    }),
  ).toEqual(['activity:mouse']);
});

test('timeline math reports pointer drag distance', () => {
  expect(
    selectionDistancePx({
      currentX: 13,
      currentY: 14,
      startX: 10,
      startY: 10,
    }),
  ).toBe(5);
});
