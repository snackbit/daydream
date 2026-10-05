import type {
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
