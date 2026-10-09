import type { LibraryRecordingDetail } from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import {
  createLibraryRedactions,
  createLibraryPlaybackSession,
  deleteLibraryRedaction,
  renameLibraryRecording,
  unredactLibraryRedactions,
} from './library-client';

const { convertFileSrcMock, invokeMock } = vi.hoisted(() => ({
  convertFileSrcMock: vi.fn(),
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: convertFileSrcMock,
  invoke: invokeMock,
}));

beforeEach(() => {
  convertFileSrcMock.mockReset();
  invokeMock.mockReset();
});
test('playback run assets reject invalid source metadata before reaching the player', async () => {
  invokeMock.mockResolvedValue({
    recordingId: 'synthetic',
    sessionId: 'session',
    error: null,
    segments: [],
    runs: [
      {
        runId: 'run',
        segmentIds: ['fragment'],
        durationMs: 30_000,
        playbackOffsetMs: 0,
        assets: {
          segmentId: 'run',
          videoUrl: 'http://127.0.0.1/video',
          thumbnails: [],
          audio: [],
          videos: [
            {
              trackId: 'source',
              streamIndex: -1,
              url: 'http://127.0.0.1/video',
              thumbnails: [],
            },
          ],
        },
      },
    ],
  });
  await expect(
    createLibraryPlaybackSession({ recordingId: 'synthetic' }),
  ).rejects.toThrow('invalid response');
});

test('createLibraryRedactions uses locked camelCase command name', async () => {
  invokeMock.mockResolvedValue({} as LibraryRecordingDetail);

  await createLibraryRedactions({
    ranges: [
      {
        endOffsetMs: 2_000,
        startOffsetMs: 1_000,
        trackIds: ['video-0'],
      },
    ],
    recordingId: 'recording-1',
  });

  expect(invokeMock).toHaveBeenCalledWith('createLibraryRedactions', {
    input: {
      ranges: [
        {
          endOffsetMs: 2_000,
          startOffsetMs: 1_000,
          trackIds: ['video-0'],
        },
      ],
      recordingId: 'recording-1',
    },
  });
});

test('deleteLibraryRedaction uses locked camelCase command name', async () => {
  invokeMock.mockResolvedValue({} as LibraryRecordingDetail);

  await deleteLibraryRedaction({
    recordingId: 'recording-1',
    redactionId: 'redaction-1',
  });

  expect(invokeMock).toHaveBeenCalledWith('deleteLibraryRedaction', {
    input: {
      recordingId: 'recording-1',
      redactionId: 'redaction-1',
    },
  });
});

test('unredactLibraryRedactions uses locked camelCase command name', async () => {
  invokeMock.mockResolvedValue({} as LibraryRecordingDetail);

  await unredactLibraryRedactions({
    ranges: [
      {
        endOffsetMs: 2_000,
        startOffsetMs: 1_000,
        trackIds: ['video-0'],
      },
    ],
    recordingId: 'recording-1',
  });

  expect(invokeMock).toHaveBeenCalledWith('unredactLibraryRedactions', {
    input: {
      ranges: [
        {
          endOffsetMs: 2_000,
          startOffsetMs: 1_000,
          trackIds: ['video-0'],
        },
      ],
      recordingId: 'recording-1',
    },
  });
});

test('renameLibraryRecording uses locked camelCase command name', async () => {
  invokeMock.mockResolvedValue({
    availability: 'available',
    durationMs: null,
    endedAtMs: null,
    id: 'recording-1',
    missingReasons: [],
    playableSegmentCount: 0,
    redactions: [],
    segments: [],
    sizeBytes: 0,
    startedAtMs: 1,
    status: 'completed',
    storageDir: '/tmp/daydream/recordings/recording-1',
    thumbnailPath: null,
    title: 'Renamed capture',
    tracks: [],
  } satisfies LibraryRecordingDetail);

  await renameLibraryRecording({
    recordingId: 'recording-1',
    title: 'Renamed capture',
  });

  expect(invokeMock).toHaveBeenCalledWith('renameLibraryRecording', {
    input: {
      recordingId: 'recording-1',
      title: 'Renamed capture',
    },
  });
});
