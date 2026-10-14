import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryRecordingSegment,
} from '@daydream/shared';
import syntheticMedia from '../../../recording-studio-route/stories/fixtures/synthetic-player.webm?url';
import { studioStoryRecording } from '../../../recording-studio-route/stories/studio-route-fixture';

// Synthetic media and public recording shapes; no captured content or devices.
export function workflowSegment(
  overrides: Partial<LibraryRecordingSegment> = {},
): LibraryRecordingSegment {
  return {
    id: 'segment',
    durationMs: 1000,
    endedAtMs: 2000,
    startedAtMs: 1000,
    fileExists: true,
    filePath: '/tmp/synthetic-workflow.mkv',
    pauseAfterMs: 0,
    playbackOffsetMs: 0,
    segmentIndex: 0,
    state: 'finalized',
    videoStreamIndex: 0,
    desktopAudioStreamIndex: null,
    microphoneAudioStreamIndex: null,
    ...overrides,
  };
}
export const workflowDetail: LibraryRecordingDetail = {
  ...studioStoryRecording,
  availability: 'available',
  missingReasons: [],
  durationMs: 1000,
  playableSegmentCount: 1,
  segments: [workflowSegment()],
  tracks: [
    {
      id: 'video-0',
      kind: 'video',
      label: 'Screen',
      description: 'Synthetic source',
      codec: 'h264',
      streamIndex: 0,
    },
  ],
};
export function workflowAssets(
  detail: LibraryRecordingDetail,
): LibraryPlaybackAssets {
  return {
    canSeekInStream: true,
    recordingId: detail.id,
    sessionId: 'synthetic-workflow',
    error: null,
    segments: detail.segments.map((segment) => ({
      segmentId: segment.id,
      videoUrl: new URL(syntheticMedia, window.location.origin).href,
      thumbnails: [],
      audio: [],
    })),
  };
}
