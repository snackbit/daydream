import type {
  LibraryPlaybackAssets,
  LibraryPlaybackThumbnails,
  LibraryRecordingDetail,
  LibraryWaveforms,
} from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { RecordingStudioScreen } from './RecordingStudioScreen';

const client = vi.hoisted(() => ({
  deleteLibraryRecording: vi.fn(),
  getLibraryRecording: vi.fn(),
  getRecordingWaveforms: vi.fn(),
  localAssetUrl: vi.fn((path: string) => `asset://${path}`),
  prepareLibraryPlaybackAssets: vi.fn(),
  prepareLibraryPlaybackThumbnails: vi.fn(),
}));

const router = vi.hoisted(() => ({
  navigate: vi.fn(),
}));

const routeState = vi.hoisted(() => ({
  recordingId: 'recording-1',
}));

vi.mock('@/features/library/library-client', () => client);

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => router.navigate,
  useParams: () => ({ recordingId: routeState.recordingId }),
}));

const detail = {
  availability: 'available',
  durationMs: 7_000,
  endedAtMs: 11_000,
  id: 'recording-1',
  missingReasons: [],
  playableSegmentCount: 2,
  segments: [
    {
      desktopAudioStreamIndex: 1,
      durationMs: 2_000,
      endedAtMs: 3_000,
      fileExists: true,
      filePath: '/tmp/daydream/recordings/recording-1/segments/0001.mkv',
      id: 'segment-1',
      microphoneAudioStreamIndex: 2,
      pauseAfterMs: 3_000,
      playbackOffsetMs: 0,
      segmentIndex: 1,
      startedAtMs: 1_000,
      state: 'finalized',
      videoStreamIndex: 0,
    },
    {
      desktopAudioStreamIndex: 1,
      durationMs: 2_000,
      endedAtMs: 8_000,
      fileExists: true,
      filePath: '/tmp/daydream/recordings/recording-1/segments/0002.mkv',
      id: 'segment-2',
      microphoneAudioStreamIndex: 2,
      pauseAfterMs: 0,
      playbackOffsetMs: 5_000,
      segmentIndex: 2,
      startedAtMs: 6_000,
      state: 'finalized',
      videoStreamIndex: 0,
    },
  ],
  sizeBytes: 1_500_000,
  startedAtMs: 1_000,
  status: 'completed',
  storageDir: '/tmp/daydream/recordings/recording-1',
  thumbnailPath: '/tmp/daydream/recordings/recording-1/segments/0001.mkv',
  tracks: [
    {
      codec: 'h264',
      description: 'Composited screen recording',
      id: 'video-0',
      kind: 'video',
      label: 'Video',
      streamIndex: 0,
    },
    {
      codec: 'opus',
      description: 'Desktop audio stream 1',
      id: 'desktop-audio-1',
      kind: 'desktop-audio',
      label: 'Desktop audio',
      streamIndex: 1,
    },
  ],
} satisfies LibraryRecordingDetail;

const waveforms = {
  error: null,
  recordingId: 'recording-1',
  tracks: [
    {
      kind: 'desktop-audio',
      segments: [
        {
          generatedAtMs: 2_000,
          maxAmplitude: 1,
          samples: [0.2, 0.8, 0.4],
          segmentId: 'segment-1',
          streamIndex: 1,
        },
      ],
      trackId: 'desktop-audio',
    },
  ],
} satisfies LibraryWaveforms;

const playbackAssets = {
  error: null,
  recordingId: 'recording-1',
  segments: [
    {
      audio: [
        {
          kind: 'desktop-audio',
          path: '/tmp/daydream/recordings/recording-1/playback/segment-1/desktop-audio-1.m4a',
          streamIndex: 1,
          trackId: 'desktop-audio-1',
        },
      ],
      segmentId: 'segment-1',
      thumbnails: [],
      videoPath:
        '/tmp/daydream/recordings/recording-1/playback/segment-1/video.mp4',
    },
    {
      audio: [
        {
          kind: 'desktop-audio',
          path: '/tmp/daydream/recordings/recording-1/playback/segment-2/desktop-audio-1.m4a',
          streamIndex: 1,
          trackId: 'desktop-audio-1',
        },
      ],
      segmentId: 'segment-2',
      thumbnails: [],
      videoPath:
        '/tmp/daydream/recordings/recording-1/playback/segment-2/video.mp4',
    },
  ],
} satisfies LibraryPlaybackAssets;

const playbackThumbnails = {
  error: null,
  recordingId: 'recording-1',
  segments: [
    {
      segmentId: 'segment-1',
      thumbnails: [
        {
          offsetMs: 0,
          path: '/tmp/daydream/recordings/recording-1/playback/segment-1/thumbs/thumb-00000.jpg',
        },
        {
          offsetMs: 1_000,
          path: '/tmp/daydream/recordings/recording-1/playback/segment-1/thumbs/thumb-00001.jpg',
        },
      ],
    },
    {
      segmentId: 'segment-2',
      thumbnails: [
        {
          offsetMs: 0,
          path: '/tmp/daydream/recordings/recording-1/playback/segment-2/thumbs/thumb-00000.jpg',
        },
      ],
    },
  ],
} satisfies LibraryPlaybackThumbnails;

beforeEach(() => {
  vi.resetAllMocks();
  routeState.recordingId = 'recording-1';
  client.deleteLibraryRecording.mockResolvedValue(undefined);
  client.getLibraryRecording.mockResolvedValue(detail);
  client.getRecordingWaveforms.mockResolvedValue(waveforms);
  client.prepareLibraryPlaybackAssets.mockResolvedValue(playbackAssets);
  client.prepareLibraryPlaybackThumbnails.mockResolvedValue(playbackThumbnails);
  router.navigate.mockResolvedValue(undefined);
});

afterEach(() => {
  vi.useRealTimers();
  cleanup();
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((nextResolve) => {
    resolve = nextResolve;
  });

  return { promise, resolve };
}

test('RecordingStudioScreen renders player controls, metadata sidebar, and tracks', async () => {
  render(<RecordingStudioScreen />);

  expect(await screen.findByText('Tracks')).toBeTruthy();
  expect(screen.getByRole('button', { name: /play/i })).toBeTruthy();
  expect(
    screen.getByRole('slider', { name: /playback position/i }),
  ).toBeTruthy();
  expect(screen.getByText('Desktop audio')).toBeTruthy();
  expect(screen.getByText('Annotation and metadata workspace')).toBeTruthy();
  expect(await screen.findByLabelText(/mute desktop audio/i)).toBeTruthy();
  expect(screen.getAllByLabelText('Audio waveform')).toHaveLength(1);
  expect(screen.getAllByLabelText('Video timeline thumbnail')).toHaveLength(3);
  expect(client.prepareLibraryPlaybackAssets).toHaveBeenCalledWith({
    recordingId: 'recording-1',
  });
  expect(client.prepareLibraryPlaybackThumbnails).toHaveBeenCalledWith({
    recordingId: 'recording-1',
  });
  await waitFor(() =>
    expect(client.localAssetUrl).toHaveBeenCalledWith(
      playbackAssets.segments[0]!.videoPath,
    ),
  );
  expect(client.localAssetUrl).toHaveBeenCalledWith(
    playbackThumbnails.segments[0]!.thumbnails[0]!.path,
  );
  expect(client.getRecordingWaveforms).toHaveBeenCalledWith({
    recordingId: 'recording-1',
  });
});

test('RecordingStudioScreen ignores stale playback assets after route changes', async () => {
  const staleAssets = deferred<LibraryPlaybackAssets>();
  const nextDetail = {
    ...detail,
    id: 'recording-2',
    segments: [
      {
        ...detail.segments[0]!,
        filePath: '/tmp/daydream/recordings/recording-2/segments/0001.mkv',
        id: 'segment-3',
        pauseAfterMs: 0,
      },
    ],
    storageDir: '/tmp/daydream/recordings/recording-2',
  } satisfies LibraryRecordingDetail;
  const nextPlaybackAssets = {
    error: null,
    recordingId: 'recording-2',
    segments: [
      {
        audio: [],
        segmentId: 'segment-3',
        thumbnails: [],
        videoPath:
          '/tmp/daydream/recordings/recording-2/playback/segment-3/video.mp4',
      },
    ],
  } satisfies LibraryPlaybackAssets;

  client.getLibraryRecording.mockImplementation((recordingId: string) =>
    Promise.resolve(recordingId === 'recording-2' ? nextDetail : detail),
  );
  client.prepareLibraryPlaybackAssets.mockImplementation(
    ({ recordingId }: { recordingId: string }) =>
      recordingId === 'recording-2'
        ? Promise.resolve(nextPlaybackAssets)
        : staleAssets.promise,
  );

  const { rerender } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(client.prepareLibraryPlaybackAssets).toHaveBeenCalledWith({
      recordingId: 'recording-1',
    }),
  );

  routeState.recordingId = 'recording-2';
  rerender(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(client.localAssetUrl).toHaveBeenCalledWith(
      nextPlaybackAssets.segments[0]!.videoPath,
    ),
  );

  staleAssets.resolve(playbackAssets);
  await Promise.resolve();

  expect(screen.queryByText('Playback preparation failed')).toBeNull();
  expect(client.localAssetUrl).not.toHaveBeenCalledWith(
    playbackAssets.segments[0]!.videoPath,
  );
});

test('RecordingStudioScreen advances immediately across back-to-back segments', async () => {
  client.getLibraryRecording.mockResolvedValue({
    ...detail,
    segments: detail.segments.map((segment, index) =>
      index === 0 ? { ...segment, pauseAfterMs: 0 } : segment,
    ),
  });

  const { container } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      playbackAssets.segments[0]!.videoPath,
    ),
  );

  fireEvent.ended(container.querySelector('video')!);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      playbackAssets.segments[1]!.videoPath,
    ),
  );
  expect(screen.queryByText(/Recording paused/i)).toBeNull();
});

test('RecordingStudioScreen waits for the actual pause gap before advancing segments', async () => {
  client.getLibraryRecording.mockResolvedValue({
    ...detail,
    segments: detail.segments.map((segment, index) =>
      index === 0 ? { ...segment, pauseAfterMs: 120 } : segment,
    ),
  });

  const { container } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      playbackAssets.segments[0]!.videoPath,
    ),
  );

  fireEvent.ended(container.querySelector('video')!);

  expect(screen.getByText(/Recording paused/i)).toBeTruthy();
  expect(container.querySelector('video')?.getAttribute('src')).toContain(
    playbackAssets.segments[0]!.videoPath,
  );

  await new Promise((resolve) => {
    window.setTimeout(resolve, 60);
  });
  expect(container.querySelector('video')?.getAttribute('src')).toContain(
    playbackAssets.segments[0]!.videoPath,
  );

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      playbackAssets.segments[1]!.videoPath,
    ),
  );
  await waitFor(() => {
    expect(screen.queryByText(/Recording paused/i)).toBeNull();
  });
});

test('RecordingStudioScreen shows disabled placeholders for missing files', async () => {
  client.getLibraryRecording.mockResolvedValue({
    ...detail,
    availability: 'missing-files',
    missingReasons: ['Segment 1 video file is missing.'],
    playableSegmentCount: 0,
    segments: detail.segments.map((segment) => ({
      ...segment,
      fileExists: false,
    })),
  });

  render(<RecordingStudioScreen />);

  expect(await screen.findAllByText('Missing video files')).toHaveLength(2);
  expect(client.prepareLibraryPlaybackAssets).not.toHaveBeenCalled();
  expect(client.prepareLibraryPlaybackThumbnails).not.toHaveBeenCalled();
  expect(client.getRecordingWaveforms).not.toHaveBeenCalled();
});

test('RecordingStudioScreen deletes a broken recording from the detail view', async () => {
  client.getLibraryRecording.mockResolvedValue({
    ...detail,
    availability: 'missing-files',
    missingReasons: ['Segment 1 video file is missing.'],
    playableSegmentCount: 0,
  });

  render(<RecordingStudioScreen />);

  fireEvent.click(await screen.findByRole('button', { name: /^delete$/i }));
  fireEvent.click(screen.getAllByRole('button', { name: /^delete$/i }).at(-1)!);

  await waitFor(() =>
    expect(client.deleteLibraryRecording).toHaveBeenCalledWith({
      recordingId: 'recording-1',
    }),
  );
  expect(router.navigate).toHaveBeenCalledWith({ to: '/library' });
});
