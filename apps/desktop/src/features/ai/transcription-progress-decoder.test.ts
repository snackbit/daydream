import { expect, test } from 'vitest';
import { decodeTranscriptionProgress } from './transcription-progress-decoder';

const source = {
  jobId: 'job',
  trackId: 'mic',
  sourceRole: 'microphone',
  status: 'paused',
  ranges: [
    { startOffsetMs: 0, endOffsetMs: 1000 },
    { startOffsetMs: 1000, endOffsetMs: 2000 },
  ],
  publishedRanges: [{ startOffsetMs: 0, endOffsetMs: 1500 }],
};
test('publication can cross adjacent chunk boundaries', () =>
  expect(decodeTranscriptionProgress([source])).toEqual([source]));
test.each([NaN, Infinity, -1, 0.5])(
  'invalid publication timestamp %s is rejected',
  (startOffsetMs) =>
    expect(() =>
      decodeTranscriptionProgress([
        { ...source, publishedRanges: [{ startOffsetMs, endOffsetMs: 1000 }] },
      ]),
    ).toThrow(),
);
test('publication outside requested coverage is rejected', () =>
  expect(() =>
    decodeTranscriptionProgress([
      { ...source, publishedRanges: [{ startOffsetMs: 0, endOffsetMs: 3000 }] },
    ]),
  ).toThrow());
