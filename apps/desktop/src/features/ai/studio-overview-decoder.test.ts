import { expect, test } from 'vitest';
import { decodeStudioOverview } from './studio-overview-decoder';

const bucket = {
  trackId: 'screen',
  kind: 'observation',
  startOffsetMs: 0,
  endOffsetMs: 1000,
  count: 3,
};
test('overview IPC retains source/count provenance without captured content', () => {
  expect(decodeStudioOverview([bucket])).toEqual([bucket]);
  expect(decodeStudioOverview([])).toEqual([]);
  for (const kind of ['speech', 'activity'])
    expect(decodeStudioOverview([{ ...bucket, kind }])[0]?.kind).toBe(kind);
});
test.each([
  null,
  {},
  [null],
  [1],
  [{ ...bucket, trackId: '' }],
  [{ ...bucket, kind: 'ocr' }],
  [{ ...bucket, startOffsetMs: -1 }],
  [{ ...bucket, startOffsetMs: 0.5 }],
  [{ ...bucket, endOffsetMs: 0 }],
  [{ ...bucket, endOffsetMs: Infinity }],
  [{ ...bucket, count: 0 }],
  [{ ...bucket, count: 1.5 }],
  [{ ...bucket, count: '3' }],
])('invalid overview input is rejected: %j', (input) => {
  expect(() => decodeStudioOverview(input)).toThrow('Invalid Studio overview.');
});
