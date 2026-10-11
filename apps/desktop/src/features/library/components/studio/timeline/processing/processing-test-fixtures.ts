import type {
  LibraryRecordingDetail,
  ProcessingChunk,
  RecordingAiProcessingState,
} from '@daydream/shared';

export const detail: LibraryRecordingDetail = {
  id: 'synthetic-processing',
  title: 'Synthetic recording',
  availability: 'available',
  status: 'completed',
  startedAtMs: 0,
  endedAtMs: 12000,
  durationMs: 10000,
  sizeBytes: 0,
  storageDir: '/synthetic',
  thumbnailPath: null,
  missingReasons: [],
  playableSegmentCount: 2,
  redactions: [],
  tracks: [
    {
      id: 'screen-a',
      kind: 'video',
      label: 'Monitor',
      description: 'Synthetic screen',
      codec: 'h264',
      streamIndex: 0,
    },
    {
      id: 'screen-b',
      kind: 'video',
      label: 'Second monitor',
      description: 'Synthetic screen',
      codec: 'h264',
      streamIndex: 3,
    },
    {
      id: 'mic',
      kind: 'microphone',
      label: 'Microphone',
      description: 'Synthetic microphone',
      codec: 'opus',
      streamIndex: 2,
    },
    {
      id: 'desktop',
      kind: 'desktop-audio',
      label: 'Desktop audio',
      description: 'Synthetic desktop',
      codec: 'opus',
      streamIndex: 1,
    },
  ],
  segments: [0, 7000].map((playbackOffsetMs, index) => ({
    id: `segment-${index}`,
    segmentIndex: index,
    playbackOffsetMs,
    durationMs: 5000,
    startedAtMs: playbackOffsetMs,
    endedAtMs: playbackOffsetMs + 5000,
    fileExists: true,
    filePath: '/synthetic.mkv',
    state: 'finalized',
    pauseAfterMs: index === 0 ? 2000 : 0,
    videoStreamIndex: 0,
    microphoneAudioStreamIndex: 2,
    desktopAudioStreamIndex: 1,
    videoStreams: [
      {
        trackId: 'screen-a',
        sourceId: 'monitor-a',
        label: 'Monitor',
        streamIndex: 0,
        width: 64,
        height: 36,
        fps: 1,
      },
      {
        trackId: 'screen-b',
        sourceId: 'monitor-b',
        label: 'Second monitor',
        streamIndex: 3,
        width: 64,
        height: 36,
        fps: 1,
      },
    ],
  })),
};
export const microphone = detail.tracks[2]!;
export const monitor = detail.tracks[0]!;
export function chunk(changes: Partial<ProcessingChunk> = {}): ProcessingChunk {
  return {
    id: 'chunk-mic',
    recordingId: detail.id,
    jobId: 'job',
    sourceKind: 'audio',
    trackId: 'mic',
    segmentId: 'segment-0',
    startOffsetMs: 0,
    endOffsetMs: 5000,
    status: 'queued',
    progress: 0,
    attemptCount: 0,
    createdAtMs: 0,
    updatedAtMs: 0,
    errorCode: null,
    resultCode: null,
    ...changes,
  };
}
export function state(chunks: ProcessingChunk[]): RecordingAiProcessingState {
  return {
    recordingId: detail.id,
    job: {
      id: 'job',
      recordingId: detail.id,
      kind: 'ai-processing',
      status: 'running',
      progress: 0,
      createdAtMs: 0,
      updatedAtMs: 0,
      errorCode: null,
    },
    chunks,
    transcript: { recordingId: detail.id, tracks: [] },
  };
}
