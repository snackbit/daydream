import type { LibraryRecordingDetail } from '@daydream/shared';

export const studioStoryRecording: LibraryRecordingDetail = {
  availability: 'missing-files',
  durationMs: 10_000,
  endedAtMs: 11_000,
  id: 'studio-story',
  missingReasons: ['Synthetic missing segment'],
  playableSegmentCount: 0,
  redactions: [],
  segments: [],
  sizeBytes: 0,
  startedAtMs: 1_000,
  status: 'completed',
  storageDir: '/tmp/synthetic-studio-story',
  thumbnailPath: null,
  title: 'Synthetic review',
  tracks: [],
};

export function deferredStory<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((accept) => {
    resolve = accept;
  });
  return { promise, resolve };
}
