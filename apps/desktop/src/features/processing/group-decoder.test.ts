import { expect, test } from 'vitest';
import {
  decodeRecordingGroupPage,
  decodeRecordingLaneDetail,
  decodeRecordingActionResult,
} from './group-decoder';
import {
  activity,
  group,
  groupPage,
  lane,
} from './testing/processing-browser-fixtures';

test('validates current activity, global counts and nullable timing without treating age as failure', () => {
  const current = groupPage(
    [
      group({
        lanes: group().lanes.map((value) => ({
          ...value,
          activity: [activity()],
        })),
      }),
    ],
    {
      counts: {
        all: 30,
        active: 20,
        attention: 4,
        history: 10,
        running: 2,
        queued: 300,
      },
    },
  );
  expect(decodeRecordingGroupPage(current)).toEqual(current);
  for (const patch of [
    { waitingReason: 'invented' },
    { status: 'succeeded' },
    { phaseStartedAtMs: -1 },
    { workerActive: 'true' },
    { startOffsetMs: 2000, endOffsetMs: 1000 },
  ]) {
    expect(() =>
      decodeRecordingLaneDetail({
        lane: { ...lane('ocr'), activity: [{ ...activity(), ...patch }] },
        units: [],
        nextCursor: null,
      }),
    ).toThrow();
  }
});

test('accepts complete native groups and reconciled groups including waveform work', () => {
  const input = groupPage([group()], { reconciledGroups: [group()] });
  expect(input.groups[0]?.lanes.map((value) => value.kind)).toEqual([
    'audio',
    'ocr',
    'vlm',
    'activity',
    'waveform',
  ]);
  expect(decodeRecordingGroupPage(input)).toEqual(input);
  const legacy = groupPage([
    group({
      lanes: group().lanes.filter((value) => value.kind !== 'waveform'),
    }),
  ]);
  expect(decodeRecordingGroupPage(legacy)).toEqual(legacy);
});

test.each(['audio', 'ocr', 'vlm', 'activity'] as const)(
  'rejects a group missing its required %s lane even when four lanes remain',
  (kind) => {
    expect(() =>
      decodeRecordingGroupPage(
        groupPage([
          group({
            lanes: group().lanes.filter((value) => value.kind !== kind),
          }),
        ]),
      ),
    ).toThrow();
  },
);

test('rejects duplicate and unknown lanes without discarding valid group data', () => {
  expect(() =>
    decodeRecordingGroupPage(
      groupPage([group({ lanes: [...group().lanes, lane('waveform')] })]),
    ),
  ).toThrow();
  expect(() =>
    decodeRecordingGroupPage({
      ...groupPage(),
      groups: [
        {
          ...group(),
          lanes: [...group().lanes, { ...lane('waveform'), kind: 'unknown' }],
        },
      ],
    }),
  ).toThrow();
});

test('waveform lanes preserve queued work and reject unknown source kinds', () => {
  const input = { lane: lane('waveform'), units: [], nextCursor: null };
  expect(decodeRecordingLaneDetail(input)).toEqual(input);
  for (const applicability of [
    'available',
    'disabled',
    'no-source',
    'protected',
    'no-eligible-input',
    'waiting',
  ] as const) {
    const value = { ...input, lane: { ...input.lane, applicability } };
    expect(decodeRecordingLaneDetail(value)).toEqual(value);
  }
  expect(() =>
    decodeRecordingLaneDetail({
      ...input,
      lane: { ...input.lane, applicability: 'unknown' },
    }),
  ).toThrow();
  expect(() =>
    decodeRecordingLaneDetail({
      ...input,
      lane: { ...input.lane, kind: 'unknown' },
    }),
  ).toThrow();
});

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
    nextCursor: { chunkId: 'older-chunk', unitId: 'older-unit' },
  };
  expect(decodeRecordingLaneDetail(value)).toEqual(value);
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

test('partial action outcomes preserve successful work and the failed job recovery code', () => {
  const value = {
    results: [
      { jobId: 'completed-job', succeeded: true, errorCode: null },
      {
        jobId: 'pending-job',
        succeeded: false,
        errorCode: 'ai_processing_resume_pending',
      },
    ],
  };
  expect(decodeRecordingActionResult(value)).toEqual(value);
  expect(() =>
    decodeRecordingActionResult({
      results: [{ jobId: 'pending-job', succeeded: 'yes', errorCode: null }],
    }),
  ).toThrow();
});
