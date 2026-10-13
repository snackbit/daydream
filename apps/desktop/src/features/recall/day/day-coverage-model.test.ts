import type { RecallCoverageInterval } from '@daydream/shared';
import { expect, test } from 'vitest';
import {
  buildDayCoverageSpans,
  coverageSourceLabel,
  coverageSpanKind,
} from './day-coverage-model';

function interval(
  startMs: number,
  endMs: number,
  kind: RecallCoverageInterval['kind'] = 'captured',
  overrides: Partial<RecallCoverageInterval> = {},
): RecallCoverageInterval {
  return {
    recordingId: 'recording',
    segmentId: `segment-${startMs}`,
    startMs,
    endMs,
    kind,
    sourceTrackIds: [],
    ...overrides,
  };
}

test('capture and interpretation overlap once and merge across adjacent segment boundaries', () => {
  const input = [
    interval(15, 30),
    interval(0, 15, 'uninterpreted'),
    interval(15, 30, 'uninterpreted'),
    interval(0, 15),
    interval(0, 15),
  ];
  const original = structuredClone(input);
  const spans = buildDayCoverageSpans(input);
  expect(spans).toEqual([
    {
      startMs: 0,
      endMs: 30,
      sources: [
        { recordingId: 'recording', kind: 'captured', sourceTrackIds: [] },
        { recordingId: 'recording', kind: 'uninterpreted', sourceTrackIds: [] },
      ],
    },
  ]);
  expect(spans.map(coverageSpanKind)).toEqual(['uninterpreted']);
  expect(input).toEqual(original);
});

test('partial interpretation splits spans without claiming unrecorded gaps', () => {
  const spans = buildDayCoverageSpans([
    interval(0, 30),
    interval(15, 30, 'uninterpreted'),
    interval(30, 45, 'gap', { recordingId: '' }),
    interval(45, 60, 'missing'),
  ]);
  expect(spans.map(({ startMs, endMs }) => [startMs, endMs])).toEqual([
    [0, 15],
    [15, 30],
    [30, 45],
    [45, 60],
  ]);
  expect(spans.map(coverageSpanKind)).toEqual([
    'captured',
    'uninterpreted',
    'gap',
    'missing',
  ]);
  expect(
    buildDayCoverageSpans([interval(0, 15), interval(30, 45)]),
  ).toHaveLength(2);
});

test('protection scope is canonicalized and changes remain visible even when row labels are similar', () => {
  const spans = buildDayCoverageSpans([
    interval(0, 40),
    interval(5, 10, 'redacted', {
      sourceTrackIds: ['video-1', 'video-0', 'video-0'],
    }),
    interval(10, 15, 'redacted', { sourceTrackIds: ['video-0', 'video-1'] }),
    interval(15, 20, 'redacted', { sourceTrackIds: ['microphone'] }),
    interval(20, 25, 'redacted'),
  ]);
  expect(spans.map(({ startMs, endMs }) => [startMs, endMs])).toEqual([
    [0, 5],
    [5, 15],
    [15, 20],
    [20, 25],
    [25, 40],
  ]);
  expect(
    spans[1]?.sources.find((source) => source.kind === 'redacted')
      ?.sourceTrackIds,
  ).toEqual(['video-0', 'video-1']);
  expect(
    coverageSourceLabel({
      recordingId: 'recording',
      kind: 'redacted',
      sourceTrackIds: [],
    }),
  ).toContain('all sources');
  expect(
    coverageSourceLabel({
      recordingId: 'recording',
      kind: 'redacted',
      sourceTrackIds: ['microphone'],
    }),
  ).toContain('microphone');
  expect(
    coverageSpanKind({
      startMs: 0,
      endMs: 1,
      sources: [{ recordingId: '', kind: 'redacted', sourceTrackIds: [] }],
    }),
  ).toBe('redacted');
});

test('concurrent and consecutive recordings preserve identity without repeating the time span', () => {
  const spans = buildDayCoverageSpans([
    interval(0, 15),
    interval(0, 15, 'captured', { recordingId: 'other' }),
    interval(15, 30, 'captured', { recordingId: 'other' }),
  ]);
  expect(spans).toHaveLength(2);
  expect(spans[0]?.sources.map((source) => source.recordingId)).toEqual([
    'other',
    'recording',
  ]);
  expect(spans[1]?.sources).toHaveLength(1);
});

test('empty and zero-length inputs have no coverage', () => {
  expect(buildDayCoverageSpans([])).toEqual([]);
  expect(buildDayCoverageSpans([interval(1, 1), interval(2, 1)])).toEqual([]);
  expect(coverageSpanKind({ startMs: 0, endMs: 0, sources: [] })).toBe('gap');
});

test.each(['captured', 'uninterpreted', 'missing', 'gap'] as const)(
  '%s coverage has a user-facing label',
  (kind) => {
    expect(
      coverageSourceLabel({ recordingId: '', kind, sourceTrackIds: [] }),
    ).toBe(
      {
        captured: 'Captured',
        uninterpreted: 'Not interpreted yet',
        missing: 'Missing source · timing unavailable',
        gap: 'No recording · activity unknown',
      }[kind],
    );
  },
);
