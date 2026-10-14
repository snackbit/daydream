import type {
  LibraryRecordingTrack,
  RecordingAiProcessingState,
  RecordingTranscriptionChunkState,
  RecordingTranscriptionTrackState,
} from '@daydream/shared';
import { expect, test } from 'vitest';
import { buildDerivedLane } from './derived-ai-lane-model';

const track: LibraryRecordingTrack = {
  id: 'microphone-2',
  kind: 'microphone',
  label: 'Microphone',
  description: '',
  codec: 'opus',
  streamIndex: 2,
};
const segment = {
  id: 'speech-a',
  segmentId: 'media-a',
  startOffsetMs: 1000,
  endOffsetMs: 1500,
  text: 'Synthetic first recording',
  words: [],
  confidence: null,
  provider: 'local-whisper-cpp',
  model: null,
};
const chunk: RecordingTranscriptionChunkState = {
  id: 'chunk-a',
  segmentId: 'media-a',
  startOffsetMs: 0,
  endOffsetMs: 2000,
  attemptCount: 1,
  status: 'succeeded',
  result: 'text',
  resultCode: null,
  errorCode: null,
  progress: 1,
  text: segment.text,
  transcriptSegments: [segment],
};
const source: RecordingTranscriptionTrackState = {
  trackKey: 'recording-a:microphone',
  trackId: 'source-a',
  libraryTrackId: track.id,
  sourceRole: 'microphone',
  label: 'Microphone',
  description: 'Synthetic source',
  chunks: [chunk],
  status: 'complete',
  statusLabel: 'Complete',
  textSegmentCount: 1,
};
const state: RecordingAiProcessingState = {
  recordingId: 'synthetic-day',
  chunks: [],
  job: null,
  transcript: { recordingId: 'synthetic-day', tracks: [] },
  transcriptionTracks: [source],
};

test('a source lane combines recordings without losing chunk identity or mutating snapshots', () => {
  const second = {
    ...source,
    trackKey: 'recording-b:microphone',
    chunks: [
      {
        ...chunk,
        id: 'chunk-b',
        segmentId: 'media-b',
        startOffsetMs: 4000,
        endOffsetMs: 6000,
      },
    ],
    textSegmentCount: 2,
  };
  const lane = buildDerivedLane({
    track,
    aiProcessingState: { ...state, transcriptionTracks: [source, second] },
  });
  expect(
    lane.chunks.map((item) => [item.id, item.segmentId, item.startOffsetMs]),
  ).toEqual([
    ['chunk-a', 'media-a', 0],
    ['chunk-b', 'media-b', 4000],
  ]);
  expect(lane.trackState?.textSegmentCount).toBe(3);
  expect(lane.status).toBe('complete');
  expect(source.chunks).toHaveLength(1);
  expect(buildDerivedLane({ track, aiProcessingState: state }).trackState).toBe(
    source,
  );
});

test.each([
  ['queued', 'in-progress'],
  ['running', 'in-progress'],
  ['failed', 'failed'],
  ['cancelled', 'cancelled'],
  ['no-speech', 'no-speech'],
  ['skipped-silence', 'no-speech'],
] as const)(
  'multiple recordings expose %s source outcomes',
  (result, status) => {
    const lane = buildDerivedLane({
      track,
      aiProcessingState: {
        ...state,
        transcriptionTracks: [
          { ...source, chunks: [] },
          {
            ...source,
            trackKey: 'recording-b',
            chunks: [{ ...chunk, result, transcriptSegments: [], text: null }],
          },
        ],
      },
    });
    expect(lane.status).toBe(status);
    expect(lane.statusLabel).not.toBe('Complete');
  },
);

test('empty and video sources keep their established states', () => {
  expect(buildDerivedLane({ track, aiProcessingState: null }).chunks).toEqual(
    [],
  );
  expect(
    buildDerivedLane({
      track,
      aiProcessingState: {
        ...state,
        transcriptionTracks: [
          { ...source, chunks: [] },
          { ...source, chunks: [] },
        ],
      },
    }).status,
  ).toBe('ready');
  expect(
    buildDerivedLane({
      track: { ...track, kind: 'video' },
      aiProcessingState: state,
    }).status,
  ).toBe('not-implemented');
});

test('legacy day sources include transcript tracks from every matching recording', () => {
  const second = {
    ...segment,
    id: 'speech-b',
    segmentId: 'media-b',
    startOffsetMs: 4000,
    endOffsetMs: 5000,
    text: 'Synthetic later recording',
  };
  const legacy: RecordingAiProcessingState = {
    ...state,
    transcriptionTracks: [],
    transcript: {
      recordingId: state.recordingId,
      tracks: [
        { trackId: 'source-a', sourceRole: 'microphone', segments: [segment] },
        { trackId: 'source-b', sourceRole: 'microphone', segments: [second] },
        {
          trackId: 'desktop-audio-1',
          sourceRole: 'desktop-audio',
          segments: [{ ...segment, text: 'Other source' }],
        },
      ],
    },
    chunks: [0, 4000].map((startOffsetMs, i) => ({
      id: `legacy-${i}`,
      segmentId: `media-${i}`,
      recordingId: state.recordingId,
      jobId: 'job',
      sourceKind: 'audio',
      trackId: track.id,
      startOffsetMs,
      endOffsetMs: startOffsetMs + 2000,
      status: 'succeeded',
      attemptCount: 1,
      progress: 1,
      errorCode: null,
      resultCode: null,
      createdAtMs: 0,
      updatedAtMs: 0,
    })),
  };
  const lane = buildDerivedLane({ track, aiProcessingState: legacy });
  expect(lane.chunks.map((item) => item.text)).toEqual([
    segment.text,
    second.text,
  ]);
  expect(lane.status).toBe('complete');
  expect(lane.trackState).toBeNull();
});

test.each([
  ['queued', null, 'in-progress'],
  ['running', null, 'in-progress'],
  ['failed', null, 'failed'],
  ['cancelled', null, 'cancelled'],
  ['succeeded', 'skipped-silence', 'no-speech'],
  ['succeeded', null, 'no-speech'],
] as const)(
  'legacy %s results remain visible without inventing transcript text',
  (status, resultCode, expected) => {
    const lane = buildDerivedLane({
      track,
      aiProcessingState: {
        ...state,
        transcriptionTracks: [],
        chunks: [
          {
            id: 'legacy-empty',
            segmentId: null,
            recordingId: state.recordingId,
            jobId: 'job',
            sourceKind: 'audio',
            trackId: track.kind,
            startOffsetMs: null,
            endOffsetMs: null,
            status,
            resultCode,
            attemptCount: 1,
            progress: 0,
            errorCode: null,
            createdAtMs: 0,
            updatedAtMs: 0,
          },
        ],
      },
    });
    expect(lane.status).toBe(expected);
    expect(lane.chunks[0]?.text).toBeNull();
  },
);
