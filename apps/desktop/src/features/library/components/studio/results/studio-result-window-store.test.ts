import { afterEach, expect, test } from 'vitest';
import { useStudioResultWindowStore as store } from './studio-result-window-store';

afterEach(() => store.setState(store.getInitialState(), true));
test('overview ranges stay bounded and stable while panning within one page', () => {
  store.getState().setWindow('day', 1000, 530_000, 0.0001);
  const range = store.getState().range;
  expect(range).toEqual({
    startOffsetMs: 0,
    endOffsetMs: 540_000,
    bucketMs: 480_000,
  });
  store.getState().setWindow('day', 2000, 535_000, 0.0001);
  expect(store.getState().range).toBe(range);
  store.getState().setWindow('day', 31_000, 100_000, 1);
  expect(store.getState().range).toEqual({
    startOffsetMs: 30_000,
    endOffsetMs: 120_000,
  });
  store.getState().setWindow('day', 31_000, 100_000, 0.005);
  expect(store.getState().range.bucketMs).toBe(10_000);
});
test('Review browsing pages are independent of timeline zoom and change ownership on navigation', () => {
  store.getState().setReview('day', 600_000);
  const page = store.getState().review;
  expect(page).toEqual({ startOffsetMs: 450_000, endOffsetMs: 810_000 });
  store.getState().setReview('day', 610_000);
  expect(store.getState().review).toBe(page);
  store.getState().setWindow('day', 5_000_000, 7_200_000, 0.0001);
  expect(store.getState().review).toBe(page);
  store.getState().setReview('recording', 0);
  expect(store.getState().reviewOwner).toBe('recording');
  expect(store.getState().review.startOffsetMs).toBe(0);
});
