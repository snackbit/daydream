import { expect, test } from 'vitest';
import {
  decodeRecallDay,
  decodeRecallDayCard,
  decodeRecallEvidence,
  decodeRecallJob,
  decodeRecallSearchResult,
  decodeRecallStateChanged,
  decodeLibraryDaysPage,
} from './recall-decoders';
import { recallDayFixture } from './testing/recall-browser-fixtures';

test('Library metadata pages decode without Recall content and reject inconsistent revisions or cursors', () => {
  const { startMs, endMs, timezone, inputFingerprint, summary, ...day } =
    recallDayFixture.day;
  void [startMs, endMs, timezone, inputFingerprint, summary];
  const page = {
    privacyRevision: day.privacyRevision,
    days: [day],
    nextBeforeDayId: day.dayId,
    generationReady: true,
    generationUnavailableReason: null,
  };
  expect(decodeLibraryDaysPage(page)).toEqual(page);
  expect(
    decodeLibraryDaysPage({ ...page, days: [], nextBeforeDayId: null }).days,
  ).toEqual([]);
  for (const invalid of [
    null,
    { ...page, days: null },
    { ...page, privacyRevision: day.privacyRevision + 1 },
    { ...page, nextBeforeDayId: '2026-01-01' },
    { ...page, days: [], nextBeforeDayId: day.dayId },
    { ...page, generationReady: undefined },
    { ...page, days: [{ ...day, recordingCount: -1 }] },
    { ...page, days: [{ ...day, status: 'unknown' }] },
  ])
    expect(() => decodeLibraryDaysPage(invalid)).toThrow('invalid response');
});

const evidence = {
  evidenceId: 'synthetic-evidence',
  recordingId: 'synthetic-recording',
  segmentId: 'synthetic-segment',
  sourceKind: 'ocr',
  sourceTrackId: 'screen:synthetic',
  sourceId: 'synthetic-occurrence',
  sourceFrameRef: 'synthetic-frame',
  documentId: 'synthetic-document',
  startOffsetMs: 10,
  endOffsetMs: 20,
  absoluteStartMs: 1000,
  absoluteEndMs: 1010,
  contentRevision: 'synthetic-revision',
  text: 'Synthetic private text',
};
test('evidence preserves exact source provenance and rejects ambiguous or reversed ranges', () => {
  expect(decodeRecallEvidence(evidence)).toEqual(evidence);
  for (const invalid of [
    { ...evidence, sourceTrackId: undefined },
    { ...evidence, startOffsetMs: 21 },
    { ...evidence, absoluteEndMs: 999 },
    { ...evidence, sourceKind: 'vision' },
    { ...evidence, startOffsetMs: NaN },
  ])
    expect(() => decodeRecallEvidence(invalid)).toThrow('invalid response');
});
test('search requires explicit semantic and indexing completeness metadata', () => {
  const result = {
    privacyRevision: 1,
    hits: [{ evidence, score: 0.5 }],
    complete: false,
    unavailableReason: 'model_missing',
    nextCursor: null,
    semanticUsed: false,
    indexedEvidenceCount: 0,
    eligibleEvidenceCount: 1,
  };
  expect(decodeRecallSearchResult(result)).toEqual(result);
  expect(() =>
    decodeRecallSearchResult({ ...result, semanticUsed: undefined }),
  ).toThrow();
  expect(() =>
    decodeRecallSearchResult({
      ...result,
      hits: [{ evidence, score: Infinity }],
    }),
  ).toThrow();
});
test('jobs and invalidation metadata reject malformed lifecycle responses', () => {
  const job = {
    id: 'synthetic-job',
    dayId: null,
    kind: 'index',
    status: 'paused',
    progress: 0.2,
    errorCode: null,
    updatedAtMs: 100,
  };
  expect(decodeRecallJob(job)).toEqual(job);
  expect(() => decodeRecallJob({ ...job, progress: 1.1 })).toThrow();
  expect(() =>
    decodeRecallStateChanged({ dayId: null, privacyRevision: -1 }),
  ).toThrow();
  expect(decodeRecallStateChanged({ dayId: null, privacyRevision: 2 })).toEqual(
    { dayId: null, privacyRevision: 2 },
  );
});
test('day nested content is validated before rendering generated sessions or tasks', () => {
  const {
    evidence: _evidence,
    recordings: _recordings,
    ...historical
  } = recallDayFixture;
  expect(decodeRecallDay(historical)).toMatchObject({
    evidence: [],
    recordings: [],
  });
  expect(() =>
    decodeRecallDay({
      day: {},
      sessions: [],
      suggestions: [],
      tasks: [],
      feedback: [],
      coverage: [],
    }),
  ).toThrow();
});
test('thumbnail preparation accepts only explicit recording identity or absence', () => {
  const day = {
    ...recallDayFixture.day,
    thumbnailRecordingId: 'synthetic-recording',
  };
  expect(decodeRecallDayCard(day)).toEqual(day);
  expect(
    decodeRecallDayCard({ ...day, thumbnailRecordingId: null })
      .thumbnailRecordingId,
  ).toBeNull();
  expect(() =>
    decodeRecallDayCard({ ...day, thumbnailRecordingId: undefined }),
  ).toThrow();
  expect(() =>
    decodeRecallDayCard({ ...day, thumbnailRecordingId: 12 }),
  ).toThrow();
});
