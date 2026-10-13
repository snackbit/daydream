import { expect, test } from 'vitest';
import { dayTimelineFixture } from '../../day-studio/day-timeline-fixtures';
import {
  buildTimelineRegions,
  continuousTimelineRegions,
} from './segment-scrubber-model';

test('continuous scrubber merges storage boundaries while preserving pauses and hover provenance', () => {
  const source = dayTimelineFixture().recordings[0]!.segments[0]!;
  const segments = Array.from({ length: 960 }, (_, index) => ({
    ...source,
    id: `part-${index}`,
    segmentIndex: index,
    playbackOffsetMs: index * 30_000,
    durationMs: 30_000,
    pauseAfterMs: 0,
  }));
  const regions = buildTimelineRegions(segments);
  expect(continuousTimelineRegions(regions)).toMatchObject([
    { startMs: 0, endMs: 28_800_000, kind: 'segment' },
  ]);
  expect(regions[0]?.endMs).toBe(30_000);
  expect(regions[959]?.segment.id).toBe('part-959');
  segments[0]!.pauseAfterMs = 1000;
  segments[1]!.playbackOffsetMs = 31_000;
  expect(
    continuousTimelineRegions(buildTimelineRegions(segments.slice(0, 2))),
  ).toMatchObject([
    { startMs: 0, endMs: 30_000, kind: 'segment' },
    { startMs: 30_000, endMs: 31_000, kind: 'pause' },
    { startMs: 31_000, endMs: 61_000, kind: 'segment' },
  ]);
  expect(continuousTimelineRegions([])).toEqual([]);
});
