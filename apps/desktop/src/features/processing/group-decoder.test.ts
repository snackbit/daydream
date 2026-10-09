import { expect, test } from 'vitest';
import {
  decodeRecordingGroupPage,
  decodeRecordingLaneDetail,
} from './group-decoder';
import { group, groupPage, lane } from './testing/processing-browser-fixtures';

test('rejects overlapping section outcomes and a fixed ETA for open recording input', () => {
  expect(() =>
    decodeRecordingGroupPage(
      groupPage([
        group({
          lanes: [
            lane('audio', { failed: 2 }),
            lane('ocr'),
            lane('vlm'),
            lane('activity'),
          ],
        }),
      ]),
    ),
  ).toThrow();
  expect(() =>
    decodeRecordingGroupPage(
      groupPage([group({ inputOpen: true, etaMs: 1000 })]),
    ),
  ).toThrow();
  expect(() =>
    decodeRecordingGroupPage(
      groupPage([
        group({
          indexCoverage: {
            applicability: 'available',
            indexed: 10,
            pending: 1,
            total: 10,
          },
        }),
      ]),
    ),
  ).toThrow();
});

test('validates section provenance and rejects an unsupported audio source role', () => {
  const value = {
    lane: lane('audio'),
    units: [
      {
        id: 'speech:chunk:0:1000',
        chunkId: 'chunk',
        sourceKind: 'audio',
        startOffsetMs: 0,
        endOffsetMs: 1000,
        status: 'succeeded',
        progress: 1,
        errorCode: null,
        attemptCount: 1,
        jobId: 'synthetic-job',
        sourceTrackId: 'microphone-2',
        sourceRole: 'microphone',
      },
    ],
    nextCursor: null,
  };
  expect(decodeRecordingLaneDetail(value).units[0]?.sourceRole).toBe(
    'microphone',
  );
  expect(() =>
    decodeRecordingLaneDetail({
      ...value,
      units: [{ ...value.units[0], sourceRole: 'video' }],
    }),
  ).toThrow();
});
