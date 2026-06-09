import type { LibraryRecordingDetail } from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import { renameLibraryRecording } from './library-client';

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

test('renameLibraryRecording uses locked camelCase command name', async () => {
  invokeMock.mockResolvedValue({
    availability: 'available',
    durationMs: null,
    endedAtMs: null,
    id: 'recording-1',
    missingReasons: [],
    playableSegmentCount: 0,
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
