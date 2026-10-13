import type {
  LibraryRecordingDetail,
  RecordingActivity,
  RecordingAiProcessingState,
  RecordingVisualObservations,
} from '@daydream/shared';
import { expect, test } from 'vitest';
import {
  visibleActivity,
  visibleAiProcessingState,
  visibleVisualObservations,
} from './derived-result-visibility';

const detail: LibraryRecordingDetail = {
  availability: 'available',
  durationMs: 3_000,
  endedAtMs: 3_000,
  id: 'recording-1',
  missingReasons: [],
  playableSegmentCount: 1,
  segments: [],
  sizeBytes: 0,
  startedAtMs: 0,
  status: 'completed',
  storageDir: '/tmp/synthetic',
  thumbnailPath: null,
  title: null,
  tracks: [
    {
      codec: null,
      description: '',
      id: 'video-0',
      kind: 'video',
      label: 'Video',
      streamIndex: 0,
    },
    {
      codec: null,
      description: '',
      id: 'desktop-audio-1',
      kind: 'desktop-audio',
      label: 'Desktop',
      streamIndex: 1,
    },
    {
      codec: null,
      description: '',
      id: 'microphone-2',
      kind: 'microphone',
      label: 'Mic',
      streamIndex: 2,
    },
  ],
  redactions: [
    {
      createdAtMs: 0,
      endOffsetMs: 2_000,
      id: 'cut',
      reason: null,
      recordingId: 'recording-1',
      startOffsetMs: 1_000,
      trackIds: ['video-0', 'desktop-audio-1', 'activity:focus'],
    },
  ],
};
const segment = {
  confidence: null,
  endOffsetMs: 1_500,
  id: 'text',
  model: 'small',
  provider: 'local-whisper-cpp',
  segmentId: 'segment-1',
  startOffsetMs: 1_000,
  text: 'synthetic private speech',
  words: [],
};

test('cached overview counts disappear immediately for newly redacted source ranges', () => {
  const buckets = ['desktop-audio-1', 'microphone-2'].map((trackId) => ({
    trackId,
    kind: 'speech' as const,
    startOffsetMs: 1000,
    endOffsetMs: 1500,
    count: 4,
  }));
  const state = {
    recordingId: detail.id,
    chunks: [],
    job: null,
    transcript: { recordingId: detail.id, tracks: [] },
    timelineOverview: buckets,
  };
  expect(visibleAiProcessingState(detail, state)?.timelineOverview).toEqual([
    buckets[1],
  ]);
  expect(
    visibleAiProcessingState(
      { ...detail, redactions: [{ ...detail.redactions[0]!, trackIds: [] }] },
      state,
    )?.timelineOverview,
  ).toEqual([]);
});
test('cached transcript UUIDs map to Library source roles and partially overlapping chunk text is hidden', () => {
  const state: RecordingAiProcessingState = {
    chunks: [],
    job: null,
    recordingId: detail.id,
    transcript: {
      recordingId: detail.id,
      tracks: [
        {
          segments: [
            segment,
            {
              ...segment,
              id: 'safe',
              startOffsetMs: 2_000,
              endOffsetMs: 2_500,
              text: 'safe speech',
            },
          ],
          sourceRole: 'desktop-audio',
          trackId: 'persisted-uuid',
        },
        { segments: [segment], sourceRole: 'microphone', trackId: 'mic-uuid' },
      ],
    },
    transcriptionTracks: [
      {
        chunks: [
          {
            attemptCount: 1,
            endOffsetMs: 3_000,
            errorCode: null,
            id: 'chunk',
            progress: 1,
            result: 'text',
            resultCode: null,
            segmentId: 'segment-1',
            startOffsetMs: 0,
            status: 'succeeded',
            text: 'synthetic private speech safe speech',
            transcriptSegments: [segment],
          },
        ],
        description: '',
        label: 'Desktop',
        libraryTrackId: 'desktop-audio-1',
        sourceRole: 'desktop-audio',
        status: 'complete',
        statusLabel: 'Complete',
        textSegmentCount: 1,
        trackId: 'persisted-uuid',
        trackKey: 'desktop-audio',
      },
    ],
  };
  const visible = visibleAiProcessingState(detail, state);
  expect(
    visible?.transcript.tracks[0]?.segments.map((item) => item.text),
  ).toEqual(['safe speech']);
  expect(visible?.transcript.tracks[1]?.segments).toEqual([segment]);
  expect(visible?.transcriptionTracks?.[0]?.chunks[0]).toMatchObject({
    text: null,
    transcriptSegments: [],
  });
  expect(state.transcriptionTracks?.[0]?.chunks[0]?.text).toBe(
    'synthetic private speech safe speech',
  );
});

test('cached OCR and VLM honor half-open video redactions while activity stays lane-scoped', () => {
  const regions = [999, 1_000, 1_999, 2_000].map((frameTimestampMs) => ({
    chunkId: 'chunk',
    confidence: null,
    frameTimestampMs,
    id: String(frameTimestampMs),
    jobId: 'job',
    model: null,
    provider: 'tesseract',
    recordingId: detail.id,
    region: { x: 0, y: 0, width: 10, height: 10 },
    sourceFrameRef: String(frameTimestampMs),
    text: 'synthetic screen',
  }));
  const visual: RecordingVisualObservations = {
    ocrRegions: regions,
    recordingId: detail.id,
    vlmObservations: regions.map((region) => ({
      ...region,
      observationKind: 'screen-observation',
      promptVersion: 'test',
      provider: 'openai-compatible',
    })),
    vlmState: {
      errorCode: null,
      providerStatus: 'configured',
      status: 'succeeded',
    },
  };
  const visible = visibleVisualObservations(detail, visual);
  expect(visible?.ocrRegions.map((item) => item.frameTimestampMs)).toEqual([
    999, 2_000,
  ]);
  expect(visible?.vlmObservations.map((item) => item.frameTimestampMs)).toEqual(
    [999, 2_000],
  );
  const activity: RecordingActivity = {
    captureStates: [],
    events: [
      {
        applicationName: 'Synthetic',
        id: 'focus',
        kind: 'focused-window',
        occurredAtMs: 1_500,
        offsetMs: 1_500,
        recordingId: detail.id,
        title: 'Synthetic private title',
      },
    ],
    intervals: [
      {
        endOffsetMs: 1_600,
        id: 'mouse',
        jobId: 'job',
        recordingId: detail.id,
        score: 1,
        sourceKind: 'mouse',
        startOffsetMs: 1_400,
        state: 'active',
      },
    ],
    recordingId: detail.id,
    windowSnapshots: [],
  };
  expect(visibleActivity(detail, activity)).toMatchObject({
    events: [],
    intervals: activity.intervals,
  });
  expect(
    visibleVisualObservations({ ...detail, id: 'other-recording' }, visual),
  ).toBeNull();
});
