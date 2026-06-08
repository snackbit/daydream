import type {
  LibraryPlaybackAssets,
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
  closeLibraryPlaybackSession: vi.fn(),
  createLibraryPlaybackSession: vi.fn(),
  deleteLibraryRecording: vi.fn(),
  getLibraryRecording: vi.fn(),
  getRecordingWaveforms: vi.fn(),
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
          streamIndex: 1,
          trackId: 'desktop-audio-1',
          url: 'http://127.0.0.1:5000/playback/session/token/segments/segment-1/audio/desktop-audio-1',
        },
      ],
      segmentId: 'segment-1',
      thumbnails: [
        {
          offsetMs: 0,
          url: 'http://127.0.0.1:5000/playback/session/token/segments/segment-1/thumbnail/0.jpg',
        },
        {
          offsetMs: 1_000,
          url: 'http://127.0.0.1:5000/playback/session/token/segments/segment-1/thumbnail/1000.jpg',
        },
      ],
      videoUrl:
        'http://127.0.0.1:5000/playback/session/token/segments/segment-1/video',
    },
    {
      audio: [
        {
          kind: 'desktop-audio',
          streamIndex: 1,
          trackId: 'desktop-audio-1',
          url: 'http://127.0.0.1:5000/playback/session/token/segments/segment-2/audio/desktop-audio-1',
        },
      ],
      segmentId: 'segment-2',
      thumbnails: [
        {
          offsetMs: 0,
          url: 'http://127.0.0.1:5000/playback/session/token/segments/segment-2/thumbnail/0.jpg',
        },
      ],
      videoUrl:
        'http://127.0.0.1:5000/playback/session/token/segments/segment-2/video',
    },
  ],
  sessionId: 'session',
} satisfies LibraryPlaybackAssets;

beforeEach(() => {
  vi.resetAllMocks();
  vi.spyOn(window.HTMLMediaElement.prototype, 'play').mockImplementation(() =>
    Promise.resolve(),
  );
  vi.spyOn(window.HTMLMediaElement.prototype, 'pause').mockImplementation(
    () => undefined,
  );
  routeState.recordingId = 'recording-1';
  client.deleteLibraryRecording.mockResolvedValue(undefined);
  client.closeLibraryPlaybackSession.mockResolvedValue(undefined);
  client.getLibraryRecording.mockResolvedValue(detail);
  client.getRecordingWaveforms.mockResolvedValue(waveforms);
  client.createLibraryPlaybackSession.mockResolvedValue(playbackAssets);
  router.navigate.mockResolvedValue(undefined);
});

afterEach(() => {
  vi.useRealTimers();
  cleanup();
  vi.restoreAllMocks();
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
  expect(client.createLibraryPlaybackSession).toHaveBeenCalledWith({
    recordingId: 'recording-1',
  });
  expect(document.querySelector('video')?.getAttribute('src')).toContain(
    playbackAssets.segments[0]!.videoUrl,
  );
  expect(document.querySelector('img')?.getAttribute('src')).toBe(
    playbackAssets.segments[0]!.thumbnails[0]!.url,
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
        videoUrl:
          'http://127.0.0.1:5000/playback/session-2/token/segments/segment-3/video',
      },
    ],
    sessionId: 'session-2',
  } satisfies LibraryPlaybackAssets;

  client.getLibraryRecording.mockImplementation((recordingId: string) =>
    Promise.resolve(recordingId === 'recording-2' ? nextDetail : detail),
  );
  client.createLibraryPlaybackSession.mockImplementation(
    ({ recordingId }: { recordingId: string }) =>
      recordingId === 'recording-2'
        ? Promise.resolve(nextPlaybackAssets)
        : staleAssets.promise,
  );

  const { rerender } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(client.createLibraryPlaybackSession).toHaveBeenCalledWith({
      recordingId: 'recording-1',
    }),
  );

  routeState.recordingId = 'recording-2';
  rerender(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(document.querySelector('video')?.getAttribute('src')).toContain(
      nextPlaybackAssets.segments[0]!.videoUrl,
    ),
  );

  staleAssets.resolve(playbackAssets);
  await Promise.resolve();

  expect(screen.queryByText('Playback preparation failed')).toBeNull();
  expect(document.querySelector('video')?.getAttribute('src')).not.toContain(
    playbackAssets.segments[0]!.videoUrl,
  );
});

test('RecordingStudioScreen closes playback sessions that resolve after unmount', async () => {
  const pendingAssets = deferred<LibraryPlaybackAssets>();
  client.createLibraryPlaybackSession.mockReturnValue(pendingAssets.promise);

  const { unmount } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(client.createLibraryPlaybackSession).toHaveBeenCalledWith({
      recordingId: 'recording-1',
    }),
  );

  unmount();
  pendingAssets.resolve(playbackAssets);

  await waitFor(() =>
    expect(client.closeLibraryPlaybackSession).toHaveBeenCalledWith({
      sessionId: playbackAssets.sessionId,
    }),
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
      playbackAssets.segments[0]!.videoUrl,
    ),
  );

  fireEvent.ended(container.querySelector('video')!);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      playbackAssets.segments[1]!.videoUrl,
    ),
  );
  expect(screen.queryByText(/Recording paused/i)).toBeNull();
});

test('RecordingStudioScreen commits a scrubbed seek and waits for canplay before resuming', async () => {
  const { container } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}?startMs=0&request=0`,
    ),
  );

  fireEvent.play(container.querySelector('video')!);
  const slider = screen.getByRole('slider', { name: /playback position/i });
  fireEvent.pointerDown(slider);
  fireEvent.input(slider, {
    target: { value: '1000' },
  });

  expect(container.querySelector('video')?.getAttribute('src')).toContain(
    `${playbackAssets.segments[0]!.videoUrl}?startMs=0&request=0`,
  );

  fireEvent.pointerUp(slider);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}?startMs=1000&request=1`,
    ),
  );
  expect(container.querySelector('audio')?.getAttribute('src')).toContain(
    `${playbackAssets.segments[0]!.audio[0]!.url}?startMs=1000&request=1`,
  );
  expect(window.HTMLMediaElement.prototype.play).not.toHaveBeenCalled();

  fireEvent.canPlay(container.querySelector('video')!);

  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalled(),
  );
});

test('RecordingStudioScreen seeks while paused without autoplaying', async () => {
  const { container } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}?startMs=0&request=0`,
    ),
  );

  fireEvent.change(screen.getByRole('slider', { name: /playback position/i }), {
    target: { value: '1000' },
  });

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}?startMs=1000&request=1`,
    ),
  );

  fireEvent.canPlay(container.querySelector('video')!);

  expect(window.HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
});

test('RecordingStudioScreen surfaces media play rejections', async () => {
  vi.mocked(window.HTMLMediaElement.prototype.play).mockRejectedValueOnce(
    new DOMException('unsupported stream', 'NotSupportedError'),
  );

  render(<RecordingStudioScreen />);

  await screen.findByText('Tracks');
  fireEvent.click(screen.getByRole('button', { name: /^play$/i }));

  expect(await screen.findByText('Playback failed')).toBeTruthy();
  expect(screen.getByText(/unsupported stream/i)).toBeTruthy();
});

test('RecordingStudioScreen replays from the beginning after the final stream ends', async () => {
  const singleSegmentDetail = {
    ...detail,
    durationMs: 2_000,
    playableSegmentCount: 1,
    segments: [{ ...detail.segments[0]!, pauseAfterMs: 0 }],
  } satisfies LibraryRecordingDetail;
  const singleSegmentAssets = {
    ...playbackAssets,
    segments: [playbackAssets.segments[0]!],
  } satisfies LibraryPlaybackAssets;
  client.getLibraryRecording.mockResolvedValue(singleSegmentDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(singleSegmentAssets);

  const { container } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}?startMs=0&request=0`,
    ),
  );

  fireEvent.ended(container.querySelector('video')!);
  fireEvent.click(screen.getByRole('button', { name: /^play$/i }));

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}?startMs=0&request=1`,
    ),
  );
  expect(window.HTMLMediaElement.prototype.play).not.toHaveBeenCalled();

  fireEvent.canPlay(container.querySelector('video')!);

  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalled(),
  );
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
      playbackAssets.segments[0]!.videoUrl,
    ),
  );

  fireEvent.ended(container.querySelector('video')!);

  expect(screen.getByText(/Recording paused/i)).toBeTruthy();
  expect(container.querySelector('video')?.getAttribute('src')).toContain(
    playbackAssets.segments[0]!.videoUrl,
  );

  await new Promise((resolve) => {
    window.setTimeout(resolve, 60);
  });
  expect(container.querySelector('video')?.getAttribute('src')).toContain(
    playbackAssets.segments[0]!.videoUrl,
  );

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      playbackAssets.segments[1]!.videoUrl,
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
  expect(client.createLibraryPlaybackSession).not.toHaveBeenCalled();
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
