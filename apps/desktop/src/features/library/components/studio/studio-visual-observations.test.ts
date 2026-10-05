import type {
  OcrTimelineFrame,
  LibraryRecordingDetail,
  OcrRegion,
  RecordingVisualObservations,
} from '@daydream/shared';
import { expect, test } from 'vitest';
import { buildStudioVisualFrames } from './studio-visual-observations';

const detail: LibraryRecordingDetail = {
  availability: 'available',
  durationMs: 7_000,
  endedAtMs: 7_000,
  id: 'recording-1',
  missingReasons: [],
  playableSegmentCount: 2,
  redactions: [],
  sizeBytes: 0,
  startedAtMs: 0,
  status: 'completed',
  storageDir: '/tmp/synthetic',
  thumbnailPath: null,
  title: null,
  tracks: [],
  segments: [0, 5_000].map((playbackOffsetMs, index) => ({
    desktopAudioStreamIndex: null,
    durationMs: 2_000,
    endedAtMs: playbackOffsetMs + 2_000,
    fileExists: true,
    filePath: '/tmp/synthetic.mkv',
    id: `segment-${index}`,
    microphoneAudioStreamIndex: null,
    pauseAfterMs: index === 0 ? 3_000 : 0,
    playbackOffsetMs,
    segmentIndex: index,
    startedAtMs: playbackOffsetMs,
    state: 'finalized',
    videoStreamIndex: 0,
  })),
};
function region(timestamp: number, suffix = ''): OcrRegion {
  return {
    chunkId: 'chunk',
    confidence: 1,
    frameTimestampMs: timestamp,
    id: `${timestamp}${suffix}`,
    jobId: 'job',
    model: 'tesseract',
    provider: 'tesseract',
    recordingId: detail.id,
    region: { x: 0, y: 0, width: 10, height: 10 },
    sourceFrameRef: `${timestamp}${suffix}`,
    text: 'synthetic',
  };
}
test('visual frames sort once in timeline order, group source regions, and stop at segment and recording boundaries', () => {
  const observations: RecordingVisualObservations = {
    ocrRegions: [
      region(5_500),
      region(1_500),
      region(500, ':region'),
      region(500),
    ],
    recordingId: detail.id,
    vlmObservations: [],
    vlmState: {
      errorCode: null,
      providerStatus: 'disabled',
      status: 'disabled',
    },
  };
  const frames = buildStudioVisualFrames(observations, detail);
  expect(
    frames.map((frame) => [frame.frameTimestampMs, frame.endTimestampMs]),
  ).toEqual([
    [500, 1_500],
    [1_500, 2_000],
    [5_500, 7_000],
  ]);
  expect(frames[0]?.sourceFrameRefs).toEqual(['500', '500:region']);
  expect(frames[0]?.ocrRegions).toHaveLength(2);
  expect(observations.ocrRegions.map((item) => item.frameTimestampMs)).toEqual([
    5_500, 1_500, 500, 500,
  ]);
});

test('lightweight successful-empty and failed OCR samples stay selectable without recording-wide word rows', () => {
  const sourceDetail: LibraryRecordingDetail = {
    ...detail,
    tracks: [
      {
        codec: 'h264',
        description: 'Composited video',
        id: 'video-0',
        kind: 'video',
        label: 'Video',
        streamIndex: 0,
      },
    ],
  };
  const sample: OcrTimelineFrame = {
    documentId: 'sample-document',
    errorCode: null,
    frameTimestampMs: 500,
    id: 'ocr-frame',
    ocrOutputCount: 1,
    sampleEndMs: 501,
    textEmpty: true,
    recordingId: detail.id,
    segmentId: 'segment-0',
    sourceFrameRef: 'opaque-success',
    sourceTrackId: 'video-0',
    status: 'succeeded',
  };
  const frames = buildStudioVisualFrames(null, sourceDetail, [
    sample,
    {
      ...sample,
      id: 'failed-frame',
      frameTimestampMs: 5_500,
      sampleEndMs: 5_501,
      textEmpty: null,
      segmentId: 'segment-1',
      sourceFrameRef: 'opaque-failure',
      status: 'failed',
      errorCode: 'ai_ocr_vision_unconfigured',
    },
  ]);
  expect(
    frames.map((frame) => [
      frame.frameTimestampMs,
      frame.segmentId,
      frame.sourceTrackId,
      frame.ocrStatus,
    ]),
  ).toEqual([
    [500, 'segment-0', 'video-0', 'succeeded'],
    [5_500, 'segment-1', 'video-0', 'failed'],
  ]);
  expect(frames.every((frame) => frame.ocrRegions.length === 0)).toBe(true);
  expect(frames.map((frame) => frame.endTimestampMs)).toEqual([2_000, 7_000]);
  expect(frames.map((frame) => frame.ocrSampleEndMs)).toEqual([501, 5_501]);
});

test('matching timestamps on different recording sources remain distinct and retain their supplied evidence bounds', () => {
  const sourceDetail: LibraryRecordingDetail = {
    ...detail,
    tracks: [0, 1].map((streamIndex) => ({
      codec: 'h264',
      description: 'Video source',
      id: `video-${streamIndex}`,
      kind: 'video',
      label: `Video ${streamIndex}`,
      streamIndex,
    })),
  };
  const samples: OcrTimelineFrame[] = [0, 1].map((streamIndex) => ({
    id: `sample-${streamIndex}`,
    documentId: `sample-${streamIndex}`,
    errorCode: null,
    frameTimestampMs: 500,
    sampleEndMs: 501,
    textEmpty: false,
    ocrOutputCount: 1,
    recordingId: detail.id,
    segmentId: 'segment-0',
    sourceFrameRef: `opaque-${streamIndex}`,
    sourceTrackId: `video-${streamIndex}`,
    status: 'succeeded',
  }));
  const frames = buildStudioVisualFrames(null, sourceDetail, samples);
  expect(frames).toHaveLength(2);
  expect(new Set(frames.map((frame) => frame.id)).size).toBe(2);
  expect(frames.map((frame) => frame.sourceTrackId)).toEqual([
    'video-0',
    'video-1',
  ]);
  expect(frames.map((frame) => frame.endTimestampMs)).toEqual([2_000, 2_000]);
  expect(frames.map((frame) => frame.ocrSampleEndMs)).toEqual([501, 501]);
});
