import { describe, expect, test } from 'vitest';
import {
  chunk,
  detail,
  microphone,
  monitor,
  state,
  visualFrame,
} from './processing-test-fixtures';
import { timelineProcessingRanges } from './timeline-processing-ranges';

describe('timeline processing coverage', () => {
  test('complete chunk samples keep completed coverage out of pending ranges across review windows', () => {
    const recording = {
      ...detail,
      durationMs: 600000,
      segments: [
        {
          ...detail.segments[0]!,
          durationMs: 600000,
          endedAtMs: 600000,
          pauseAfterMs: 0,
        },
      ],
    };
    const processing = state([
      chunk({
        id: 'ocr',
        sourceKind: 'ocr',
        trackId: monitor.id,
        endOffsetMs: 600000,
        status: 'running',
      }),
      chunk({
        id: 'vlm',
        sourceKind: 'vlm',
        trackId: monitor.id,
        startOffsetMs: 300000,
        endOffsetMs: 500000,
      }),
      chunk({
        id: 'outside',
        sourceKind: 'ocr',
        trackId: monitor.id,
        startOffsetMs: 900000,
        endOffsetMs: 1000000,
      }),
    ]);
    processing.visualFrames = [
      visualFrame({ id: 'done-start' }),
      visualFrame({
        id: 'running',
        frameTimestampMs: 300000,
        status: 'running',
        progress: 0.5,
      }),
      visualFrame({ id: 'done-later', frameTimestampMs: 500000 }),
      visualFrame({
        id: 'queued',
        frameTimestampMs: 580000,
        status: 'queued',
        progress: 0,
      }),
    ];
    const outputs = [
      { start: 0, end: 360000 },
      { start: 450000, end: 810000 },
    ].map((review) => {
      // Both payloads include complete frames for chunks overlapping the fixed
      // viewport (300–340 s) or Review, as required by the native read contract.
      const chunks = processing.chunks.filter(
        (chunk) =>
          chunk.startOffsetMs !== null &&
          chunk.endOffsetMs !== null &&
          ((chunk.startOffsetMs < 340000 && chunk.endOffsetMs > 300000) ||
            (chunk.startOffsetMs < review.end &&
              chunk.endOffsetMs > review.start)),
      );
      return timelineProcessingRanges({
        detail: recording,
        track: monitor,
        state: { ...processing, chunks },
      });
    });
    expect(
      outputs[0]?.map((range) => [
        range.startOffsetMs,
        range.endOffsetMs,
        range.operation,
        range.status,
      ]),
    ).toEqual([
      [300000, 500000, 'ocr', 'running'],
      [300000, 500000, 'screen-observations', 'queued'],
      [580000, 600000, 'ocr', 'queued'],
    ]);
    expect(outputs[1]).toEqual(outputs[0]);
  });
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
    processing.visualFrames[0] = {
      ...processing.visualFrames[0]!,
      status: 'queued',
      progress: 0,
    };
    expect(
      timelineProcessingRanges({
        detail,
        track: monitor,
        state: processing,
      })[0],
    ).toMatchObject({ startOffsetMs: 0, endOffsetMs: 1000, status: 'queued' });
    for (const status of [
      'failed',
      'cancelled',
      'completed-with-errors',
    ] as const) {
      processing.chunks[0] = { ...ocr, status };
      expect(
        timelineProcessingRanges({ detail, track: monitor, state: processing })
          .filter((r) => r.operation === 'ocr')
          .map((r) => r.status),
      ).toEqual(
        Array(3).fill(status === 'completed-with-errors' ? 'failed' : status),
      );
    }
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
    for (const status of [
      'paused',
      'failed',
      'cancelled',
      'completed-with-errors',
    ] as const) {
      processing.job = { ...processing.job!, status };
      expect(
        timelineProcessingRanges({
          detail,
          track: microphone,
          state: processing,
        })[0]?.status,
      ).toBe(status === 'completed-with-errors' ? 'failed' : status);
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
    processing.chunks = [
      chunk({ endOffsetMs: 2000, status: 'succeeded' }),
      chunk({
        id: 'later',
        startOffsetMs: 6000,
        endOffsetMs: 7000,
        status: 'succeeded',
      }),
    ];
    expect(
      timelineProcessingRanges({
        detail,
        track: microphone,
        state: processing,
      }).map((range) => [range.startOffsetMs, range.endOffsetMs]),
    ).toEqual([[2000, 4000]]);
    processing.reprocessingScopes[0]!.status = 'succeeded';
    expect(
      timelineProcessingRanges({
        detail,
        track: microphone,
        state: processing,
      }),
    ).toEqual([]);
  });
  test('legacy transcript-only payloads and native source IDs preserve independent audio coverage', () => {
    const processing = state([]);
    processing.transcriptionTracks = [
      {
        chunks: [
          { ...chunk(), result: 'queued', text: null, transcriptSegments: [] },
        ],
        description: 'Synthetic microphone',
        label: 'Microphone',
        libraryTrackId: 'mic',
        sourceRole: 'microphone',
        status: 'in-progress',
        statusLabel: 'Queued',
        textSegmentCount: 0,
        trackId: 'native-microphone',
        trackKey: 'microphone',
      },
    ];
    for (const chunks of [
      [],
      [chunk({ trackId: 'native-microphone' })],
      [chunk({ trackId: 'microphone' })],
    ]) {
      processing.chunks = chunks;
      expect(
        timelineProcessingRanges({
          detail,
          track: microphone,
          state: processing,
        }).map((range) => [
          range.startOffsetMs,
          range.endOffsetMs,
          range.status,
        ]),
      ).toEqual([[0, 5000, 'queued']]);
      expect(
        timelineProcessingRanges({ detail, track: monitor, state: processing }),
      ).toEqual([]);
    }
    processing.transcriptionTracks[0]!.libraryTrackId = null;
    processing.chunks = [];
    expect(
      timelineProcessingRanges({
        detail,
        track: microphone,
        state: processing,
      }),
    ).toHaveLength(1);
    processing.chunks = [
      chunk({ id: 'legacy-screen', sourceKind: 'ocr', trackId: null }),
    ];
    expect(
      timelineProcessingRanges({ detail, track: monitor, state: processing })[0]
        ?.operation,
    ).toBe('ocr');
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
