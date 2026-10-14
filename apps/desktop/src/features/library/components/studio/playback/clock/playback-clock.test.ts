import { expect, test } from 'vitest';
import { driftCorrection } from './playback-clock';

test('ordinary drift uses bounded rates without periodic seeks', () => {
  expect(driftCorrection(0.3, 0, null)).toEqual({
    rate: 1.03,
    seek: false,
    majorSinceMs: null,
  });
  expect(driftCorrection(-0.3, 0, null).rate).toBe(0.97);
  expect(driftCorrection(0.01, 0, null).rate).toBe(1);
});
test('only sustained major drift receives hard correction and recovery resets the timer', () => {
  expect(driftCorrection(1.5, 0, null)).toEqual({
    rate: 1.03,
    seek: false,
    majorSinceMs: 0,
  });
  expect(driftCorrection(1.5, 1999, 0).seek).toBe(false);
  expect(driftCorrection(1.5, 2000, 0).seek).toBe(true);
  expect(driftCorrection(0.2, 2500, 0).majorSinceMs).toBeNull();
});
