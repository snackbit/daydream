import { expect, test } from 'vitest';
import { timelineThumbnailWindow } from './timeline-thumbnail-window';

const input = {
  baseUrl: 'http://127.0.0.1/synthetic/thumbnail',
  streamIndex: 2,
  segmentStartMs: 0,
  mediaStartMs: 0,
  durationMs: 28_800_000,
  startMs: 28_740_000,
  endMs: 28_800_000,
  pixelsPerMs: 0.02,
  thumbnailWidth: 120,
};
test('an eight-hour recording requests only the visible thumbnail slots', () => {
  const previews = timelineThumbnailWindow(input);
  expect(previews).toHaveLength(10);
  expect(previews[0]).toEqual({
    offsetMs: 28_740_000,
    sampleOffsetMs: 28_740_000,
    url: input.baseUrl + '/28740000.jpg/2',
  });
  expect(
    previews.every(
      (preview) =>
        preview.offsetMs >= input.startMs && preview.offsetMs < input.endMs,
    ),
  ).toBe(true);
});
test('calendar slices retain physical source time and sparse views reuse the preceding preview', () => {
  const previews = timelineThumbnailWindow({
    ...input,
    segmentStartMs: 1000,
    mediaStartMs: 5000,
    durationMs: 1000,
    startMs: 1100,
    endMs: 1500,
    pixelsPerMs: 1,
  });
  expect(previews[0]?.url).toBe(input.baseUrl + '/5000.jpg/2');
  expect(previews[0]?.sampleOffsetMs).toBe(0);
  expect(previews[0]?.offsetMs).toBe(100);
  expect(timelineThumbnailWindow({ ...input, endMs: input.startMs })).toEqual(
    [],
  );
  expect(timelineThumbnailWindow({ ...input, streamIndex: null })[0]?.url).toBe(
    input.baseUrl + '/28740000.jpg',
  );
});
