import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryWaveformTiles,
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
  getRecordingWaveformTiles: vi.fn(),
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

const waveformTiles = {
  binMs: 10,
  error: null,
  recordingId: 'recording-1',
  tracks: [
    {
      kind: 'desktop-audio',
      scaleAmplitude: 1,
      tiles: [
        {
          binMs: 10,
          durationMs: 2_000,
          generatedAtMs: 2_000,
          peakSamples: [0.2, 0.8, 0.4],
          rmsSamples: [0.1, 0.4, 0.2],
          segmentId: 'segment-1',
          streamIndex: 1,
          timelineStartMs: 0,
        },
      ],
      trackId: 'desktop-audio',
    },
  ],
} satisfies LibraryWaveformTiles;

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

const longDetail = {
  ...detail,
  durationMs: 300_000,
  endedAtMs: 301_000,
  playableSegmentCount: 1,
  segments: [
    {
      ...detail.segments[0]!,
      durationMs: 300_000,
      endedAtMs: 301_000,
      id: 'segment-long',
      pauseAfterMs: 0,
    },
  ],
} satisfies LibraryRecordingDetail;

const longPlaybackAssets = {
  ...playbackAssets,
  segments: [
    {
      ...playbackAssets.segments[0]!,
      segmentId: 'segment-long',
      thumbnails: [
        {
          offsetMs: 0,
          url: 'http://127.0.0.1:5000/playback/session/token/segments/segment-long/thumbnail/0.jpg',
        },
      ],
      videoUrl:
        'http://127.0.0.1:5000/playback/session/token/segments/segment-long/video',
    },
  ],
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
  client.getRecordingWaveformTiles.mockResolvedValue(waveformTiles);
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

function mockScrubberRect(slider: HTMLElement) {
  slider.getBoundingClientRect = () =>
    ({
      bottom: 27,
      height: 27,
      left: 0,
      right: 700,
      toJSON: () => ({}),
      top: 0,
      width: 700,
      x: 0,
      y: 0,
    }) as DOMRect;
}

test('RecordingStudioScreen renders player controls, metadata sidebar, and tracks', async () => {
  render(<RecordingStudioScreen />);

  expect(await screen.findByText('Tracks')).toBeTruthy();
  expect(screen.getByRole('button', { name: /play/i })).toBeTruthy();
  expect(
    screen.getByRole('slider', { name: /playback position/i }),
  ).toBeTruthy();
  expect(
    document.querySelectorAll('[data-playback-region="segment"]'),
  ).toHaveLength(2);
  expect(
    document.querySelectorAll('[data-playback-region="pause"]'),
  ).toHaveLength(1);
  expect(
    document.querySelectorAll('[data-segment-boundary="start"]'),
  ).toHaveLength(2);
  expect(screen.getByText('Desktop audio')).toBeTruthy();
  expect(screen.getByText('Annotation and metadata workspace')).toBeTruthy();
  expect(await screen.findByLabelText(/mute desktop audio/i)).toBeTruthy();
  expect(await screen.findByLabelText('Audio waveform')).toBeTruthy();
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
  await waitFor(() =>
    expect(client.getRecordingWaveformTiles).toHaveBeenCalledWith(
      expect.objectContaining({
        recordingId: 'recording-1',
      }),
    ),
  );
});

test('RecordingStudioScreen renders a zoomable track ruler', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);

  render(<RecordingStudioScreen />);

  expect(await screen.findByLabelText('Time ruler')).toBeTruthy();
  expect(screen.getByText('167%')).toBeTruthy();

  const timeline = screen.getByLabelText('Track timeline');
  const timelineRect = timeline.getBoundingClientRect();
  fireEvent.wheel(timeline, {
    clientX: timelineRect.left + 480,
    ctrlKey: true,
    deltaY: -100,
  });

  await waitFor(() => expect(screen.getByText('208%')).toBeTruthy());

  fireEvent.click(screen.getByRole('button', { name: /zoom out tracks/i }));

  await waitFor(() => expect(screen.getByText('167%')).toBeTruthy());
});

test('RecordingStudioScreen queues visible waveform chunks in FIFO order', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);

  render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(
      client.getRecordingWaveformTiles.mock.calls.length,
    ).toBeGreaterThanOrEqual(4),
  );
  expect(
    client.getRecordingWaveformTiles.mock.calls.slice(0, 4).map(([input]) => ({
      endTimelineMs: input.endTimelineMs,
      startTimelineMs: input.startTimelineMs,
    })),
  ).toEqual([
    { endTimelineMs: 15_000, startTimelineMs: 0 },
    { endTimelineMs: 30_000, startTimelineMs: 15_000 },
    { endTimelineMs: 45_000, startTimelineMs: 30_000 },
    { endTimelineMs: 60_000, startTimelineMs: 45_000 },
  ]);
});

test('RecordingStudioScreen commits ruler seeks through the playback pipeline', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);

  const { container } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${longPlaybackAssets.segments[0]!.videoUrl}?startMs=0&request=0`,
    ),
  );

  const timeline = screen.getByLabelText('Track timeline');
  const pointerClientX = timeline.getBoundingClientRect().left + 420;
  fireEvent.pointerDown(timeline, {
    button: 0,
    clientX: pointerClientX,
    pointerId: 1,
  });
  fireEvent.pointerUp(timeline, {
    button: 0,
    clientX: pointerClientX,
    pointerId: 1,
  });

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${longPlaybackAssets.segments[0]!.videoUrl}?startMs=`,
    ),
  );
  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      '&request=1',
    ),
  );
  const startMs = new URL(
    container.querySelector('video')?.getAttribute('src') ?? '',
  ).searchParams.get('startMs');
  expect(Number(startMs)).toBeGreaterThan(0);
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

test('RecordingStudioScreen preserves playback intent when pause fires before segment end', async () => {
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

  const video = container.querySelector('video')!;
  fireEvent.play(video);
  fireEvent.pause(video);
  fireEvent.ended(video);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      playbackAssets.segments[1]!.videoUrl,
    ),
  );

  vi.mocked(window.HTMLMediaElement.prototype.play).mockClear();
  fireEvent.canPlay(container.querySelector('video')!);

  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalled(),
  );
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
  mockScrubberRect(slider);
  fireEvent.pointerDown(slider, { clientX: 0, pointerId: 1 });
  fireEvent.pointerMove(slider, { clientX: 100, pointerId: 1 });

  expect(container.querySelector('video')?.getAttribute('src')).toContain(
    `${playbackAssets.segments[0]!.videoUrl}?startMs=0&request=0`,
  );

  fireEvent.pointerUp(slider, { clientX: 100, pointerId: 1 });

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

  const slider = screen.getByRole('slider', { name: /playback position/i });
  mockScrubberRect(slider);
  fireEvent.pointerDown(slider, { clientX: 100, pointerId: 1 });
  fireEvent.pointerUp(slider, { clientX: 100, pointerId: 1 });

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}?startMs=1000&request=1`,
    ),
  );

  fireEvent.canPlay(container.querySelector('video')!);

  expect(window.HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
});

test('RecordingStudioScreen shows scrubber thumbnail previews and paused gaps', async () => {
  render(<RecordingStudioScreen />);

  await screen.findByText('Tracks');
  const slider = screen.getByRole('slider', { name: /playback position/i });
  mockScrubberRect(slider);

  fireEvent.pointerEnter(slider, { clientX: 100, pointerId: 1 });
  fireEvent.pointerMove(slider, { clientX: 100, pointerId: 1 });

  expect(await screen.findByText('Segment 1 - 0:01')).toBeTruthy();
  expect(document.querySelector('img[src*="thumbnail/1000.jpg"]')).toBeTruthy();

  fireEvent.pointerMove(slider, { clientX: 300, pointerId: 1 });

  expect(await screen.findAllByText('Paused')).toHaveLength(2);
  expect(screen.getByText(/0:03 gap/i)).toBeTruthy();
});

test('RecordingStudioScreen jumps paused-gap seeks to the next segment', async () => {
  const { container } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}?startMs=0&request=0`,
    ),
  );

  const slider = screen.getByRole('slider', { name: /playback position/i });
  mockScrubberRect(slider);
  fireEvent.pointerDown(slider, { clientX: 300, pointerId: 1 });
  fireEvent.pointerUp(slider, { clientX: 300, pointerId: 1 });

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[1]!.videoUrl}?startMs=0&request=1`,
    ),
  );
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

  fireEvent.play(container.querySelector('video')!);
  fireEvent.ended(container.querySelector('video')!);

  expect(screen.getByText(/Recording paused/i)).toBeTruthy();
  expect(screen.getByRole('button', { name: /play now/i })).toBeTruthy();
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
  vi.mocked(window.HTMLMediaElement.prototype.play).mockClear();
  fireEvent.canPlay(container.querySelector('video')!);
  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalled(),
  );
  await waitFor(() => {
    expect(screen.queryByText(/Recording paused/i)).toBeNull();
  });
});

test('RecordingStudioScreen play now skips the pause gap immediately', async () => {
  client.getLibraryRecording.mockResolvedValue({
    ...detail,
    segments: detail.segments.map((segment, index) =>
      index === 0 ? { ...segment, pauseAfterMs: 30_000 } : segment,
    ),
  });

  const { container } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      playbackAssets.segments[0]!.videoUrl,
    ),
  );

  fireEvent.play(container.querySelector('video')!);
  fireEvent.ended(container.querySelector('video')!);
  fireEvent.click(screen.getByRole('button', { name: /play now/i }));

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      playbackAssets.segments[1]!.videoUrl,
    ),
  );
  vi.mocked(window.HTMLMediaElement.prototype.play).mockClear();
  fireEvent.canPlay(container.querySelector('video')!);
  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalled(),
  );
  expect(screen.queryByText(/Recording paused/i)).toBeNull();
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
  expect(client.getRecordingWaveformTiles).not.toHaveBeenCalled();
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
