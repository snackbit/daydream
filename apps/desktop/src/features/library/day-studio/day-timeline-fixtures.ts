import type {
  LibraryDayTimeline,
  LibraryRecordingDetail,
} from '@daydream/shared';

/** Synthetic timestamps and streams; no captured user content. */
function recording(
  id: string,
  start: number,
  offset: number,
  monitors: number,
): LibraryRecordingDetail {
  const videoStreams = Array.from({ length: monitors }, (_, index) => ({
    trackId: `${id}-screen-${index}`,
    sourceId: `monitor-${index}`,
    label: `Monitor ${index + 1}`,
    streamIndex: index,
    width: 640,
    height: 360,
    fps: 2,
  }));
  return {
    id,
    title: `Recording ${id}`,
    startedAtMs: start,
    endedAtMs: start + 10000,
    availability: 'available',
    status: 'completed',
    durationMs: 10000,
    missingReasons: [],
    playableSegmentCount: 1,
    sizeBytes: 100,
    storageDir: '/tmp/synthetic-day',
    thumbnailPath: null,
    tracks: [
      ...videoStreams.map((v) => ({
        id: v.trackId,
        label: v.label,
        kind: 'video' as const,
        description: 'Synthetic monitor',
        codec: 'h264',
        streamIndex: v.streamIndex,
      })),
      {
        id: 'desktop-audio-1',
        label: 'Desktop audio',
        kind: 'desktop-audio',
        description: '',
        codec: 'aac',
        streamIndex: monitors,
      },
      {
        id: 'microphone-2',
        label: 'Microphone',
        kind: 'microphone',
        description: '',
        codec: 'aac',
        streamIndex: monitors + 1,
      },
    ],
    segments: [
      {
        id: `${id}-segment`,
        startedAtMs: start,
        endedAtMs: start + 10000,
        durationMs: 10000,
        playbackOffsetMs: offset,
        pauseAfterMs: 0,
        fileExists: true,
        filePath: '/tmp/synthetic-day.mkv',
        state: 'finalized',
        segmentIndex: 0,
        videoStreamIndex: 0,
        videoStreams,
        desktopAudioStreamIndex: monitors,
        microphoneAudioStreamIndex: monitors + 1,
        captureRunId: null,
      },
    ],
    redactions: [],
  };
}
export function dayTimelineFixture(): LibraryDayTimeline {
  return {
    dayId: '2026-10-11',
    startMs: 0,
    endMs: 200000,
    privacyRevision: 1,
    recordings: [recording('a', 99000, 20000, 2), recording('b', 110000, 0, 2)],
    spans: [
      {
        recordingId: 'a',
        segmentId: 'a-segment',
        startMs: 100000,
        durationMs: 5000,
        sourceStartMs: 1000,
        sourceOffsetMs: 21000,
      },
      {
        recordingId: 'b',
        segmentId: 'b-segment',
        startMs: 110000,
        durationMs: 8000,
        sourceStartMs: 0,
        sourceOffsetMs: 0,
      },
    ],
  };
}
