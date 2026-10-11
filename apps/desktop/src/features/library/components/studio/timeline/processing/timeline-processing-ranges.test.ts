import { describe, expect, test } from 'vitest';
import {
  chunk,
  detail,
  microphone,
  monitor,
  state,
} from './processing-test-fixtures';
import { timelineProcessingRanges } from './timeline-processing-ranges';

describe('timeline processing coverage', () => {
  test('shows automatic queued and running audio work, clears completed work and respects independent sources', () => {
    const queued = chunk(),
      running = chunk({
        id: 'later',
        startOffsetMs: 7000,
        endOffsetMs: 12000,
        status: 'running',
        progress: 0.4,
      }),
      desktop = chunk({ id: 'desktop', trackId: 'desktop' });
    const ranges = timelineProcessingRanges({
      detail,
      track: microphone,
      state: state([queued, running, desktop]),
    });
    expect(
      ranges.map((r) => [r.startOffsetMs, r.endOffsetMs, r.status, r.progress]),
    ).toEqual([
      [0, 5000, 'queued', 0],
      [7000, 12000, 'running', 0.4],
    ]);
    expect(
      timelineProcessingRanges({
        detail,
        track: microphone,
        state: state([
          { ...queued, status: 'succeeded' },
          { ...running, status: 'succeeded' },
        ]),
      }),
    ).toEqual([]);
  });
  test('fills video work before frames exist and consumes completed samples independently for OCR and observations', () => {
    const ocr = chunk({ id: 'ocr', sourceKind: 'ocr', trackId: 'screen-a' }),
      vlm = chunk({ id: 'vlm', sourceKind: 'vlm', trackId: 'screen-a' });
    const processing = state([
      ocr,
      vlm,
      chunk({ id: 'other-screen', sourceKind: 'ocr', trackId: 'screen-b' }),
    ]);
    expect(
      timelineProcessingRanges({
        detail,
        track: monitor,
        state: processing,
      }).map((r) => r.operation),
    ).toEqual(['ocr', 'screen-observations']);
    processing.visualFrames = [0, 1000, 3000].map(
      (frameTimestampMs, index) => ({
        id: `frame-${index}`,
        chunkId: 'ocr',
        jobId: 'job',
        recordingId: detail.id,
        segmentId: 'segment-0',
        sourceKind: 'ocr',
        sourceFrameRef: `synthetic:stream:screen-a:${index}`,
        frameTimestampMs,
        status: index === 0 ? 'succeeded' : index === 1 ? 'running' : 'queued',
        progress: 0,
        errorCode: null,
        outputCount: 0,
        region: { x: 0, y: 0, width: 1, height: 1 },
        createdAtMs: 0,
        updatedAtMs: 0,
      }),
    );
    const ranges = timelineProcessingRanges({
      detail,
      track: monitor,
      state: processing,
    });
    expect(
      ranges
        .filter((r) => r.operation === 'ocr')
        .map((r) => [r.startOffsetMs, r.endOffsetMs, r.status]),
    ).toEqual([
      [1000, 3000, 'running'],
      [3000, 5000, 'queued'],
    ]);
    expect(
      ranges.find((r) => r.operation === 'screen-observations'),
    ).toMatchObject({ startOffsetMs: 0, endOffsetMs: 5000 });
    processing.chunks[0] = { ...ocr, status: 'failed' };
    expect(
      timelineProcessingRanges({ detail, track: monitor, state: processing })
        .filter((r) => r.operation === 'ocr')
        .map((r) => r.status),
    ).toEqual(['failed', 'failed']);
  });
  test('never paints over pauses, unavailable media or hidden source ranges', () => {
    const hidden = {
      ...detail,
      segments: [
        detail.segments[0]!,
        { ...detail.segments[1]!, fileExists: false },
      ],
      redactions: [
        {
          id: 'hidden',
          recordingId: detail.id,
          startOffsetMs: 2000,
          endOffsetMs: 3000,
          trackIds: ['mic'],
          reason: 'synthetic',
          createdAtMs: 0,
        },
      ],
    };
    expect(
      timelineProcessingRanges({
        detail: hidden,
        track: microphone,
        state: state([chunk({ endOffsetMs: 12000 })]),
      }).map((r) => [r.startOffsetMs, r.endOffsetMs]),
    ).toEqual([
      [0, 2000],
      [3000, 5000],
    ]);
  });
  test('parent pause and failure stop queued chunks from appearing active', () => {
    const processing = state([chunk()]);
    for (const status of ['paused', 'failed', 'cancelled'] as const) {
      processing.job = { ...processing.job!, status };
      expect(
        timelineProcessingRanges({
          detail,
          track: microphone,
          state: processing,
        })[0]?.status,
      ).toBe(status);
    }
  });
  test('pending admission and persisted scopes show work even before chunks exist, without reviving completed jobs', () => {
    const scope = {
      id: 'scope',
      trackId: 'mic',
      sourceLabel: 'Microphone',
      operation: 'transcription' as const,
      startOffsetMs: 1000,
      endOffsetMs: 4000,
    };
    expect(
      timelineProcessingRanges({
        detail,
        track: microphone,
        state: null,
        plannedScopes: [scope],
      })[0],
    ).toMatchObject({
      status: 'planned',
      startOffsetMs: 1000,
      endOffsetMs: 4000,
    });
    const processing = state([]);
    processing.reprocessingScopes = [
      { ...scope, jobId: 'job', status: 'queued' },
    ];
    expect(
      timelineProcessingRanges({
        detail,
        track: microphone,
        state: processing,
      })[0]?.status,
    ).toBe('queued');
    processing.reprocessingScopes[0]!.status = 'succeeded';
    expect(
      timelineProcessingRanges({
        detail,
        track: microphone,
        state: processing,
      }),
    ).toEqual([]);
  });
  test('unknown timing and another recording never invent work', () => {
    expect(
      timelineProcessingRanges({
        detail,
        track: microphone,
        state: state([chunk({ startOffsetMs: null, endOffsetMs: null })]),
      }),
    ).toEqual([]);
    expect(
      timelineProcessingRanges({
        detail,
        track: microphone,
        state: { ...state([chunk()]), recordingId: 'different' },
      }),
    ).toEqual([]);
  });
});
