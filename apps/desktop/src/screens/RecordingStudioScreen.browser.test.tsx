import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryWaveformTiles,
  RecordingActivity,
  RecordingAiProcessingState,
  RecordingVisualObservations,
  OcrDocument,
  ProcessingJob,
  OcrTimelineFramePage,
} from '@daydream/shared';
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  within,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { useStudioTimelineStore } from '@/features/library/components/studio/studio-timeline-store';
import {
  TRACK_LABEL_WIDTH_PX,
  TRACK_LANE_HEIGHT_PX,
  TRACK_RULER_HEIGHT_PX,
} from '@/features/library/components/studio/track-timeline-math';
import { RecordingStudioScreen } from './RecordingStudioScreen';

const client = vi.hoisted(() => ({
  closeLibraryPlaybackSession: vi.fn(),
  createLibraryPlaybackSession: vi.fn(),
  createLibraryRedactions: vi.fn(),
  deleteLibraryRecording: vi.fn(),
  deleteLibraryRedaction: vi.fn(),
  getLibraryRecordingDebug: vi.fn(),
  getLibraryRecording: vi.fn(),
  getRecordingWaveformTiles: vi.fn(),
  renameLibraryRecording: vi.fn(),
  unredactLibraryRedactions: vi.fn(),
}));

const aiClient = vi.hoisted(() => ({
  getRecordingAiProcessingState: vi.fn(),
  getRecordingActivity: vi.fn(),
  getRecordingVisualObservations: vi.fn(),
  listenToAiProcessingChanged: vi.fn(),
  startRecordingAiProcessing: vi.fn(),
  retryProcessingJob: vi.fn(),
  resumeProcessingJob: vi.fn(),
}));

const ocrClient = vi.hoisted(() => ({
  getOcrTimelineFrames: vi.fn(),
  getOcrDocument: vi.fn(),
  getOcrHistory: vi.fn(),
  reprocessRecordingOcr: vi.fn(),
}));

const router = vi.hoisted(() => ({
  navigate: vi.fn(),
}));

const routeState = vi.hoisted(() => ({
  recordingId: 'recording-1',
}));

let originalViewport: { height: number; width: number } | null = null;
let layoutStylesheet: HTMLLinkElement | null = null;

vi.mock('@/features/library/library-client', () => client);
vi.mock('@/features/ai/ai-client', () => aiClient);
vi.mock('@/features/ai/ocr/ocr-client', () => ocrClient);
vi.mock('@/features/settings/use-debug-enabled', () => ({
  useDebugEnabled: () => false,
}));

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
  redactions: [],
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
  title: null,
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

const allTimelineLaneIds = [
  'activity:focus',
  'activity:keyboard',
  'activity:mouse',
  'desktop-audio-1',
  'video-0',
];

function legacyOcrDocument(text: string, timestampMs: number): OcrDocument {
  return {
    blocks: [
      {
        confidence: null,
        id: 'legacy-block',
        order: 0,
        rawRegionIds: [],
        region: null,
        text,
        words: [],
      },
    ],
    cacheHit: false,
    configFingerprint: 'legacy',
    contentId: null,
    engine: 'legacy',
    errorCode: null,
    frameTimestampMs: timestampMs,
    height: null,
    id: `legacy-${timestampMs}`,
    model: 'tesseract',
    observedEndMs: timestampMs,
    observedStartMs: timestampMs,
    occurrenceCount: 1,
    provenance: 'reconstructed',
    provider: 'tesseract',
    recordingId: detail.id,
    segmentId: 'segment-1',
    sourceFrameRef: 'legacy-opaque-ref',
    sourceTrackId: 'video-0',
    status: 'succeeded',
    supportTimestampsMs: [timestampMs],
    text,
    width: null,
  };
}

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

beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  ocrClient.getOcrDocument.mockResolvedValue({
    document: null,
    recordingId: detail.id,
    sourceTrackId: 'video-0',
  });
  ocrClient.getOcrHistory.mockResolvedValue({ items: [], nextCursor: null });
  ocrClient.getOcrTimelineFrames.mockResolvedValue({
    recordingId: detail.id,
    frames: [],
    nextCursor: null,
  });
  ocrClient.reprocessRecordingOcr.mockResolvedValue(undefined);
  aiClient.retryProcessingJob.mockResolvedValue(undefined);
  aiClient.resumeProcessingJob.mockResolvedValue(undefined);
  useStudioTimelineStore.setState(
    useStudioTimelineStore.getInitialState(),
    true,
  );
  vi.spyOn(window.HTMLMediaElement.prototype, 'play').mockImplementation(() =>
    Promise.resolve(),
  );
  vi.spyOn(window.HTMLMediaElement.prototype, 'pause').mockImplementation(
    () => undefined,
  );
  routeState.recordingId = 'recording-1';
  client.deleteLibraryRecording.mockResolvedValue(undefined);
  client.createLibraryRedactions.mockResolvedValue(detail);
  client.deleteLibraryRedaction.mockResolvedValue(detail);
  client.unredactLibraryRedactions.mockResolvedValue(detail);
  client.closeLibraryPlaybackSession.mockResolvedValue(undefined);
  client.getLibraryRecording.mockResolvedValue(detail);
  client.renameLibraryRecording.mockImplementation(
    ({ title }: { recordingId: string; title: string | null }) =>
      Promise.resolve({
        ...detail,
        title,
      }),
  );
  client.getRecordingWaveformTiles.mockResolvedValue(waveformTiles);
  client.createLibraryPlaybackSession.mockResolvedValue(playbackAssets);
  aiClient.getRecordingAiProcessingState.mockResolvedValue({
    chunks: [],
    job: null,
    recordingId: 'recording-1',
    transcript: { recordingId: 'recording-1', tracks: [] },
  });
  aiClient.getRecordingActivity.mockResolvedValue({
    captureStates: [],
    events: [],
    intervals: [],
    recordingId: 'recording-1',
    windowSnapshots: [],
  });
  aiClient.getRecordingVisualObservations.mockResolvedValue({
    ocrRegions: [],
    recordingId: 'recording-1',
    vlmObservations: [],
    vlmState: {
      errorCode: null,
      providerStatus: 'disabled',
      status: 'disabled',
    },
  });
  aiClient.listenToAiProcessingChanged.mockResolvedValue(() => undefined);
  aiClient.startRecordingAiProcessing.mockResolvedValue({
    createdAtMs: 1,
    errorCode: null,
    id: 'job-1',
    kind: 'ai-processing',
    progress: 0,
    recordingId: 'recording-1',
    status: 'queued',
    updatedAtMs: 1,
  });
  router.navigate.mockResolvedValue(undefined);
});

afterEach(async () => {
  vi.useRealTimers();
  cleanup();
  if (originalViewport) {
    const viewport = originalViewport;
    originalViewport = null;
    await page.viewport(viewport.width, viewport.height);
  }
  layoutStylesheet?.remove();
  layoutStylesheet = null;
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

async function setTestViewport(width: number, height: number) {
  originalViewport ??= { height: window.innerHeight, width: window.innerWidth };
  await page.viewport(width, height);
}

async function selectStudioPanel(name: 'Inspector' | 'Timeline') {
  const tab = screen.getByRole('tab', { name });
  fireEvent.mouseDown(tab, { button: 0, ctrlKey: false });
  fireEvent.mouseUp(tab);
  fireEvent.click(tab);
  await waitFor(() => expect(tab.getAttribute('aria-selected')).toBe('true'));
}

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

function trackTimelineViewport(timeline: HTMLElement): HTMLElement {
  const viewport = timeline.closest('[data-slot="scroll-area-viewport"]');
  expect(viewport).toBeTruthy();

  return viewport as HTMLElement;
}

function mockTimelineScrollMetrics(viewport: HTMLElement) {
  let scrollLeft = 0;
  Object.defineProperty(viewport, 'scrollLeft', {
    configurable: true,
    get: () => scrollLeft,
    set: (value: number) => {
      scrollLeft = value;
    },
  });
  Object.defineProperty(viewport, 'clientWidth', {
    configurable: true,
    value: 800,
  });
  Object.defineProperty(viewport, 'scrollWidth', {
    configurable: true,
    value: 2_400,
  });
}

function mockTimelineRect(timeline: HTMLElement) {
  timeline.getBoundingClientRect = () =>
    ({
      bottom: 260,
      height: 260,
      left: 0,
      right: 900,
      toJSON: () => ({}),
      top: 0,
      width: 900,
      x: 0,
      y: 0,
    }) as DOMRect;
}

function timelineClientXForMs(
  timeline: HTMLElement,
  timelineMs: number,
  totalDurationMs = detail.durationMs,
) {
  const ruler = screen.getByLabelText('Time ruler') as HTMLElement;
  const timelineWidthPx = Number.parseFloat(ruler.style.width);
  if (!Number.isFinite(timelineWidthPx) || timelineWidthPx <= 0) {
    throw new Error('Timeline ruler width is unavailable for the test.');
  }

  return (
    timeline.getBoundingClientRect().left +
    TRACK_LABEL_WIDTH_PX +
    (timelineMs / totalDurationMs) * timelineWidthPx
  );
}

async function mockPlaybackStartAt(timeline: HTMLElement, timelineMs: number) {
  // Let ResizeObserver settle the rendered ruler width before calculating the
  // pointer coordinates; pointer events otherwise race the initial viewport.
  await new Promise<void>((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
  );
  const x = timelineClientXForMs(timeline, timelineMs, 300_000);
  const lane = timeline.querySelector(
    '[data-lane-content][data-track-id="activity:mouse"]',
  );
  if (!(lane instanceof HTMLElement)) {
    throw new Error('Expected the mouse activity lane.');
  }

  fireEvent.pointerDown(lane, {
    button: 0,
    clientX: x,
    clientY: 1,
    pointerId: 44,
  });
  fireEvent.pointerUp(lane, {
    button: 0,
    clientX: x,
    clientY: 1,
    pointerId: 44,
  });
}

function dispatchTrackWheel(
  timeline: HTMLElement,
  init: WheelEventInit,
): WheelEvent {
  const event = new WheelEvent('wheel', {
    bubbles: true,
    cancelable: true,
    ...init,
  });
  fireEvent(timeline, event);

  return event;
}

function dragTimelineRange({
  clientY,
  ctrlKey = false,
  pointerId,
  shiftKey = false,
  target,
  timeline,
  xValues,
}: {
  clientY: number;
  ctrlKey?: boolean;
  pointerId: number;
  shiftKey?: boolean;
  target: HTMLElement;
  timeline: HTMLElement;
  xValues: number[];
}) {
  const [startX, ...moveXValues] = xValues;
  expect(startX).toBeDefined();

  fireEvent.pointerDown(target, {
    button: 0,
    clientX: startX,
    clientY,
    ctrlKey,
    pointerId,
    shiftKey,
  });

  for (const clientX of moveXValues) {
    fireEvent.pointerMove(timeline, {
      button: 0,
      clientX,
      clientY,
      ctrlKey,
      pointerId,
      shiftKey,
    });
  }

  fireEvent.pointerUp(timeline, {
    button: 0,
    clientX: moveXValues[moveXValues.length - 1] ?? startX,
    clientY,
    ctrlKey,
    pointerId,
    shiftKey,
  });
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
  expect(screen.getByRole('heading', { name: 'Inspector' })).toBeTruthy();
  expect(
    screen.getByRole('heading', { name: 'Recording details' }),
  ).toBeTruthy();
  expect(screen.getByText('recording-1')).toBeTruthy();
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

test('RecordingStudioScreen refresh reconciles new tracks and clips', async () => {
  client.getLibraryRecording
    .mockResolvedValueOnce(detail)
    .mockResolvedValueOnce({
      ...detail,
      segments: [
        ...detail.segments,
        {
          ...detail.segments[1]!,
          id: 'segment-3',
          playbackOffsetMs: 7_000,
          segmentIndex: 3,
        },
      ],
      tracks: [
        ...detail.tracks,
        {
          codec: 'opus',
          description: 'Microphone',
          id: 'microphone-2',
          kind: 'microphone',
          label: 'Microphone',
          streamIndex: 2,
        },
      ],
    } satisfies LibraryRecordingDetail);

  render(<RecordingStudioScreen />);

  expect(await screen.findByText('Desktop audio')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /refresh/i }));

  expect(
    await screen.findByRole('button', { name: /^Select Microphone$/i }),
  ).toBeTruthy();
  expect(
    screen.getByRole('button', { name: /^Microphone Segment 3$/i }),
  ).toBeTruthy();
  expect(screen.getByText('3 segments')).toBeTruthy();
});

test('RecordingStudioScreen starts AI processing from the detail header', async () => {
  render(<RecordingStudioScreen />);

  expect(await screen.findByText('Tracks')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /process ai/i }));

  await waitFor(() =>
    expect(aiClient.startRecordingAiProcessing).toHaveBeenCalledWith({
      recordingId: 'recording-1',
    }),
  );
});

test('RecordingStudioScreen shows processed AI state without a spinner', async () => {
  aiClient.getRecordingAiProcessingState.mockResolvedValue({
    chunks: [
      {
        attemptCount: 1,
        createdAtMs: 1,
        endOffsetMs: 2_000,
        errorCode: null,
        id: 'chunk-1',
        jobId: 'job-1',
        progress: 1,
        recordingId: 'recording-1',
        resultCode: null,
        segmentId: 'segment-1',
        sourceKind: 'audio',
        startOffsetMs: 0,
        status: 'succeeded',
        trackId: 'desktop-audio-1',
        updatedAtMs: 2,
      },
    ],
    job: {
      createdAtMs: 1,
      errorCode: null,
      id: 'job-1',
      kind: 'ai-processing',
      progress: 1,
      recordingId: 'recording-1',
      status: 'succeeded',
      updatedAtMs: 2,
    },
    recordingId: 'recording-1',
    transcript: {
      recordingId: 'recording-1',
      tracks: [],
    },
  });

  render(<RecordingStudioScreen />);

  const processedButton = await screen.findByRole('button', {
    name: /processed/i,
  });
  expect((processedButton as HTMLButtonElement).disabled).toBe(true);
  expect(screen.queryByText('Transcription')).toBeNull();
  expect(screen.queryByText('No speech detected')).toBeNull();
  expect(screen.queryByText('OCR + VLM')).toBeNull();
});

test('RecordingStudioScreen clears stale AI state after route changes', async () => {
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
  aiClient.getRecordingAiProcessingState.mockImplementation(
    ({ recordingId }: { recordingId: string }) =>
      recordingId === 'recording-2'
        ? Promise.reject(new Error('AI state unavailable'))
        : Promise.resolve({
            chunks: [],
            job: {
              createdAtMs: 1,
              errorCode: null,
              id: 'job-1',
              kind: 'ai-processing',
              progress: 1,
              recordingId: 'recording-1',
              status: 'succeeded',
              updatedAtMs: 2,
            },
            recordingId: 'recording-1',
            transcript: {
              recordingId: 'recording-1',
              tracks: [],
            },
          }),
  );
  client.getLibraryRecording.mockImplementation((recordingId: string) =>
    Promise.resolve(recordingId === 'recording-2' ? nextDetail : detail),
  );

  const { rerender } = render(<RecordingStudioScreen />);

  const processedButton = await screen.findByRole('button', {
    name: /processed/i,
  });
  expect((processedButton as HTMLButtonElement).disabled).toBe(true);

  routeState.recordingId = 'recording-2';
  rerender(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(screen.getAllByText('recording-2').length).toBeGreaterThan(0),
  );

  const processButton = screen.getByRole('button', { name: /process ai/i });
  expect((processButton as HTMLButtonElement).disabled).toBe(false);
  expect(screen.queryByRole('button', { name: /processed/i })).toBeNull();
});

test('RecordingStudioScreen treats active chunks as processing even when the job is succeeded', async () => {
  aiClient.getRecordingAiProcessingState.mockResolvedValue({
    chunks: [
      {
        attemptCount: 1,
        createdAtMs: 1,
        endOffsetMs: 2_000,
        errorCode: null,
        id: 'chunk-1',
        jobId: 'job-1',
        progress: 0.5,
        recordingId: 'recording-1',
        resultCode: null,
        segmentId: 'segment-1',
        sourceKind: 'audio',
        startOffsetMs: 0,
        status: 'running',
        trackId: 'desktop-audio-1',
        updatedAtMs: 2,
      },
    ],
    job: {
      createdAtMs: 1,
      errorCode: null,
      id: 'job-1',
      kind: 'ai-processing',
      progress: 1,
      recordingId: 'recording-1',
      status: 'succeeded',
      updatedAtMs: 2,
    },
    recordingId: 'recording-1',
    transcript: {
      recordingId: 'recording-1',
      tracks: [],
    },
  });

  render(<RecordingStudioScreen />);

  const processingButton = await screen.findByRole('button', {
    name: /processing/i,
  });

  expect((processingButton as HTMLButtonElement).disabled).toBe(true);
  expect(screen.queryByRole('button', { name: /processed/i })).toBeNull();
  expect(screen.getByLabelText('AI processing in progress')).toBeTruthy();
});

test('RecordingStudioScreen updates AI lanes from processing events', async () => {
  let processingHandler = (_state: RecordingAiProcessingState) => {
    throw new Error('processing handler was not registered');
  };
  aiClient.listenToAiProcessingChanged.mockImplementation((handler) => {
    processingHandler = handler;
    return Promise.resolve(() => undefined);
  });

  render(<RecordingStudioScreen />);

  expect(await screen.findByLabelText('Track timeline')).toBeTruthy();
  expect(screen.queryByText('Transcription')).toBeNull();
  expect(screen.queryByText('OCR + VLM')).toBeNull();

  processingHandler({
    chunks: [
      {
        attemptCount: 1,
        createdAtMs: 1,
        endOffsetMs: 2_000,
        errorCode: null,
        id: 'chunk-1',
        jobId: 'job-1',
        progress: 0.05,
        recordingId: 'recording-1',
        resultCode: null,
        segmentId: 'segment-1',
        sourceKind: 'audio',
        startOffsetMs: 0,
        status: 'running',
        trackId: 'desktop-audio-1',
        updatedAtMs: 2,
      },
    ],
    job: {
      createdAtMs: 1,
      errorCode: null,
      id: 'job-1',
      kind: 'ai-processing',
      progress: 0.05,
      recordingId: 'recording-1',
      status: 'running',
      updatedAtMs: 2,
    },
    recordingId: 'recording-1',
    transcript: { recordingId: 'recording-1', tracks: [] },
  });

  expect(
    await screen.findByLabelText('AI processing in progress'),
  ).toBeTruthy();
  expect(screen.queryByText('Transcription in progress')).toBeNull();
  expect(
    (screen.getByRole('button', { name: /processing/i }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);

  processingHandler({
    chunks: [
      {
        attemptCount: 1,
        createdAtMs: 1,
        endOffsetMs: 2_000,
        errorCode: null,
        id: 'chunk-1',
        jobId: 'job-1',
        progress: 1,
        recordingId: 'recording-1',
        resultCode: null,
        segmentId: 'segment-1',
        sourceKind: 'audio',
        startOffsetMs: 0,
        status: 'succeeded',
        trackId: 'desktop-audio-1',
        updatedAtMs: 3,
      },
      {
        attemptCount: 1,
        createdAtMs: 1,
        endOffsetMs: 7_000,
        errorCode: 'ai_processing_audio_extract_failed',
        id: 'chunk-2',
        jobId: 'job-1',
        progress: 0,
        recordingId: 'recording-1',
        resultCode: null,
        segmentId: 'segment-2',
        sourceKind: 'audio',
        startOffsetMs: 5_000,
        status: 'failed',
        trackId: 'desktop-audio-1',
        updatedAtMs: 3,
      },
    ],
    job: {
      createdAtMs: 1,
      errorCode: 'ai_processing_audio_extract_failed',
      id: 'job-1',
      kind: 'ai-processing',
      progress: 0.5,
      recordingId: 'recording-1',
      status: 'failed',
      updatedAtMs: 3,
    },
    recordingId: 'recording-1',
    transcript: {
      recordingId: 'recording-1',
      tracks: [
        {
          segments: [
            {
              confidence: null,
              endOffsetMs: 1_200,
              id: 'transcript-1',
              model: 'small',
              provider: 'local-whisper-cpp',
              segmentId: 'segment-1',
              startOffsetMs: 500,
              text: 'Standup notes',
              words: [],
            },
          ],
          sourceRole: 'desktop-audio',
          trackId: 'desktop-audio-1',
        },
      ],
    },
  });

  expect(await screen.findByText('Standup notes')).toBeTruthy();
  expect(screen.queryByText('Transcription failed')).toBeNull();
  expect(screen.getByLabelText('AI processing failed')).toBeTruthy();
  expect(
    (screen.getByRole('button', { name: /retry ai/i }) as HTMLButtonElement)
      .disabled,
  ).toBe(false);
});

test('RecordingStudioScreen explains local Whisper GPU failures in the inspector', async () => {
  aiClient.getRecordingAiProcessingState.mockResolvedValue({
    chunks: [
      {
        attemptCount: 1,
        createdAtMs: 1,
        endOffsetMs: 2_000,
        errorCode: 'ai_transcription_provider_failed',
        id: 'chunk-1',
        jobId: 'job-1',
        progress: 0,
        recordingId: 'recording-1',
        resultCode: null,
        segmentId: 'segment-1',
        sourceKind: 'audio',
        startOffsetMs: 0,
        status: 'failed',
        trackId: 'desktop-audio-1',
        updatedAtMs: 2,
      },
    ],
    job: {
      createdAtMs: 1,
      errorCode: 'ai_transcription_provider_failed',
      id: 'job-1',
      kind: 'ai-processing',
      progress: 0,
      recordingId: 'recording-1',
      status: 'failed',
      updatedAtMs: 2,
    },
    recordingId: 'recording-1',
    transcript: { recordingId: 'recording-1', tracks: [] },
    transcriptionTracks: [
      {
        chunks: [
          {
            attemptCount: 1,
            endOffsetMs: 2_000,
            errorCode: 'ai_transcription_provider_failed',
            id: 'chunk-1',
            progress: 0,
            result: 'failed',
            resultCode: null,
            segmentId: 'segment-1',
            startOffsetMs: 0,
            status: 'failed',
            text: null,
            transcriptSegments: [],
          },
        ],
        description: 'desktop-audio timeline track desktop-audio-1',
        label: 'Desktop audio transcription',
        libraryTrackId: 'desktop-audio-1',
        sourceRole: 'desktop-audio',
        status: 'failed',
        statusLabel: 'Transcription failed',
        textSegmentCount: 0,
        trackId: 'desktop-audio-1',
        trackKey: 'desktop-audio',
      },
    ],
  });

  render(<RecordingStudioScreen />);

  fireEvent.click(
    await screen.findByRole('button', { name: /^Select Desktop audio$/i }),
  );

  expect(
    await screen.findByText(/Local Whisper transcription failed/i),
  ).toBeTruthy();
  expect(
    screen.getByText(/switch local transcription backend to CPU/i),
  ).toBeTruthy();
  expect(screen.getByText('ai_transcription_provider_failed')).toBeTruthy();
});

test('RecordingStudioScreen shows visual frame details from video AI markers', async () => {
  ocrClient.getOcrDocument.mockResolvedValue({
    document: legacyOcrDocument('Checkout Total', 500),
    recordingId: detail.id,
    sourceTrackId: 'video-0',
  });
  aiClient.getRecordingVisualObservations.mockResolvedValue({
    ocrRegions: [
      {
        chunkId: 'ocr-chunk-1',
        confidence: 0.9,
        frameTimestampMs: 500,
        id: 'ocr-1',
        jobId: 'job-1',
        model: 'tesseract',
        provider: 'tesseract',
        recordingId: 'recording-1',
        region: {
          height: 20,
          width: 120,
          x: 10,
          y: 20,
        },
        sourceFrameRef: 'segment:segment-1:offset:500:frame:0',
        text: 'Checkout',
      },
      {
        chunkId: 'ocr-chunk-1',
        confidence: 0.86,
        frameTimestampMs: 500,
        id: 'ocr-2',
        jobId: 'job-1',
        model: 'tesseract',
        provider: 'tesseract',
        recordingId: 'recording-1',
        region: {
          height: 20,
          width: 80,
          x: 140,
          y: 20,
        },
        sourceFrameRef: 'segment:segment-1:offset:500:frame:0:region:140,20',
        text: 'Total',
      },
      {
        chunkId: 'ocr-chunk-1',
        confidence: 0.8,
        frameTimestampMs: 1_000,
        id: 'ocr-3',
        jobId: 'job-1',
        model: 'tesseract',
        provider: 'tesseract',
        recordingId: 'recording-1',
        region: {
          height: 20,
          width: 90,
          x: 20,
          y: 50,
        },
        sourceFrameRef: 'segment:segment-1:offset:1000:frame:1',
        text: 'Payment',
      },
    ],
    recordingId: 'recording-1',
    vlmObservations: [
      {
        chunkId: 'vlm-chunk-1',
        confidence: null,
        frameTimestampMs: 500,
        id: 'vlm-1',
        jobId: 'job-1',
        model: 'qwen/qwen3-vl-8b',
        observationKind: 'screen-observation',
        promptVersion: 'm2-visual-observation-v2',
        provider: 'openai-compatible',
        recordingId: 'recording-1',
        region: {
          height: 1440,
          width: 2560,
          x: 0,
          y: 0,
        },
        sourceFrameRef: 'segment:segment-1:offset:500:frame:0',
        text: 'A browser checkout page is visible with a payment form and order summary.',
      },
      {
        chunkId: 'vlm-chunk-1',
        confidence: null,
        frameTimestampMs: 1_500,
        id: 'vlm-2',
        jobId: 'job-1',
        model: 'qwen/qwen3-vl-8b',
        observationKind: 'screen-observation',
        promptVersion: 'm2-visual-observation-v2',
        provider: 'openai-compatible',
        recordingId: 'recording-1',
        region: {
          height: 1440,
          width: 2560,
          x: 0,
          y: 0,
        },
        sourceFrameRef: 'segment:segment-1:offset:1500:frame:2',
        text: 'The checkout page remains visible with the payment step active.',
      },
    ],
    vlmState: {
      errorCode: null,
      providerStatus: 'configured',
      status: 'succeeded',
    },
  } satisfies RecordingVisualObservations);

  render(<RecordingStudioScreen />);

  const markers = await screen.findAllByRole('button', {
    name: /visual frame/i,
  });
  expect(markers).toHaveLength(3);

  const marker = screen.getByRole('button', {
    name: /visual frame 0\.5s to 1\.0s, 1 vlm, 2 ocr candidates/i,
  });
  const ocrMarker = screen.getByRole('button', {
    name: /visual frame 1\.0s to 1\.5s, no vlm, 1 ocr candidate/i,
  });
  const vlmMarker = screen.getByRole('button', {
    name: /visual frame 1\.5s to 2\.0s, 1 vlm, 0 ocr candidates/i,
  });
  expect(marker.textContent).toBe('0.5s');
  expect(marker.textContent).not.toMatch(/OCR 2|VLM 1/i);
  expect(marker.getAttribute('data-visual-frame-source')).toBe('mixed');
  expect(ocrMarker.textContent).toBe('0:01');
  expect(ocrMarker.getAttribute('data-visual-frame-source')).toBe('ocr');
  expect(vlmMarker.textContent).toBe('1.5s');
  expect(vlmMarker.getAttribute('data-visual-frame-source')).toBe('vlm');

  fireEvent.click(screen.getByRole('button', { name: /^play$/i }));
  expect(await screen.findByRole('button', { name: /^pause$/i })).toBeTruthy();

  fireEvent.click(marker);

  await waitFor(() =>
    expect(
      screen
        .getByRole('slider', { name: /playback position/i })
        .getAttribute('aria-valuenow'),
    ).toBe('500'),
  );
  expect(screen.getByRole('button', { name: /^play$/i })).toBeTruthy();
  expect(
    await screen.findByText(
      'A browser checkout page is visible with a payment form and order summary.',
    ),
  ).toBeTruthy();
  expect(screen.queryByLabelText('Recognized text')).toBeNull();
  fireEvent.click(await screen.findByRole('button', { name: 'Read OCR text' }));
  expect((await screen.findByLabelText('Recognized text')).textContent).toBe(
    'Checkout Total',
  );
  await userEvent.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(screen.queryByText('Frame reference')).toBeNull();
});

test('RecordingStudioScreen renders activity lanes and seeks from focus markers', async () => {
  aiClient.getRecordingActivity.mockResolvedValue({
    captureStates: [],
    events: [
      {
        applicationName: 'Editor',
        id: 'focus-1',
        kind: 'focused-window',
        occurredAtMs: 2_200,
        offsetMs: 1_200,
        recordingId: 'recording-1',
        title: 'Notes.md',
      },
    ],
    intervals: [
      {
        endOffsetMs: 1_500,
        id: 'mouse-1',
        jobId: 'job-1',
        recordingId: 'recording-1',
        score: 1,
        sourceKind: 'mouse',
        startOffsetMs: 500,
        state: 'active',
      },
      {
        endOffsetMs: 1_800,
        id: 'keyboard-1',
        jobId: 'job-1',
        recordingId: 'recording-1',
        score: 1,
        sourceKind: 'keyboard',
        startOffsetMs: 900,
        state: 'active',
      },
    ],
    recordingId: 'recording-1',
    windowSnapshots: [],
  } satisfies RecordingActivity);

  render(<RecordingStudioScreen />);

  expect(
    await screen.findByRole('button', { name: /^Select Mouse$/i }),
  ).toBeTruthy();
  expect(
    screen.getByRole('button', { name: /^Select Keyboard$/i }),
  ).toBeTruthy();
  expect(
    screen.getByRole('button', { name: /^Select Activity$/i }),
  ).toBeTruthy();
  expect(
    screen.getByRole('button', { name: /Mouse activity.*0:01 to 0:02/i }),
  ).toBeTruthy();
  expect(
    screen.getByRole('button', { name: /Keyboard activity.*0:01 to 0:02/i }),
  ).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /^play$/i }));
  expect(await screen.findByRole('button', { name: /^pause$/i })).toBeTruthy();

  fireEvent.click(
    screen.getByRole('button', {
      name: /Focus changed at 0:01 - Editor - Notes\.md/i,
    }),
  );

  await waitFor(() =>
    expect(
      screen
        .getByRole('slider', { name: /playback position/i })
        .getAttribute('aria-valuenow'),
    ).toBe('1200'),
  );
  expect(screen.getByRole('button', { name: /^play$/i })).toBeTruthy();
});

test('RecordingStudioScreen reloads activity after creating redactions', async () => {
  ocrClient.getOcrDocument
    .mockResolvedValueOnce({
      document: legacyOcrDocument('private screen secret', 1_700),
      recordingId: detail.id,
      sourceTrackId: 'video-0',
    })
    .mockRejectedValue(new Error('Reload unavailable'));
  const redactedDetail = {
    ...detail,
    redactions: [
      {
        createdAtMs: 12_000,
        endOffsetMs: 4_500,
        id: 'redaction-1',
        reason: null,
        recordingId: 'recording-1',
        startOffsetMs: 1_500,
        trackIds: allTimelineLaneIds,
      },
    ],
  } satisfies LibraryRecordingDetail;
  aiClient.getRecordingAiProcessingState
    .mockResolvedValueOnce({
      chunks: [
        {
          attemptCount: 1,
          createdAtMs: 1,
          endOffsetMs: 2_000,
          errorCode: null,
          id: 'private-chunk',
          jobId: 'private-job',
          progress: 1,
          recordingId: detail.id,
          resultCode: null,
          segmentId: 'segment-1',
          sourceKind: 'audio',
          startOffsetMs: 0,
          status: 'succeeded',
          trackId: 'desktop-audio-1',
          updatedAtMs: 1,
        },
      ],
      job: null,
      recordingId: detail.id,
      transcript: {
        recordingId: detail.id,
        tracks: [
          {
            sourceRole: 'desktop-audio',
            trackId: 'stored-audio-uuid',
            segments: [
              {
                confidence: null,
                endOffsetMs: 1_800,
                id: 'private-text',
                model: 'small',
                provider: 'local-whisper-cpp',
                segmentId: 'segment-1',
                startOffsetMs: 1_700,
                text: 'private spoken secret',
                words: [],
              },
            ],
          },
        ],
      },
    } satisfies RecordingAiProcessingState)
    .mockRejectedValue(new Error('reload unavailable'));
  aiClient.getRecordingVisualObservations
    .mockResolvedValueOnce({
      recordingId: detail.id,
      ocrRegions: [
        {
          chunkId: 'private-ocr',
          confidence: 1,
          frameTimestampMs: 1_700,
          id: 'private-region',
          jobId: 'private-job',
          model: 'tesseract',
          provider: 'tesseract',
          recordingId: detail.id,
          region: { x: 0, y: 0, width: 100, height: 20 },
          sourceFrameRef: 'segment:segment-1:offset:1700:frame:1',
          text: 'private screen secret',
        },
      ],
      vlmObservations: [],
      vlmState: {
        errorCode: null,
        providerStatus: 'disabled',
        status: 'disabled',
      },
    } satisfies RecordingVisualObservations)
    .mockRejectedValue(new Error('reload unavailable'));
  aiClient.getRecordingActivity
    .mockResolvedValueOnce({
      captureStates: [],
      events: [
        {
          applicationName: 'Editor',
          id: 'focus-1',
          kind: 'focused-window',
          occurredAtMs: 2_200,
          offsetMs: 1_700,
          recordingId: 'recording-1',
          title: 'Notes.md',
        },
      ],
      intervals: [],
      recordingId: 'recording-1',
      windowSnapshots: [],
    } satisfies RecordingActivity)
    .mockResolvedValue({
      captureStates: [],
      events: [],
      intervals: [],
      recordingId: 'recording-1',
      windowSnapshots: [],
    } satisfies RecordingActivity);
  client.createLibraryRedactions.mockResolvedValue(redactedDetail);
  render(<RecordingStudioScreen />);

  expect(
    await screen.findByRole('button', {
      name: /Focus changed at 0:02 - Editor - Notes\.md/i,
    }),
  ).toBeTruthy();
  expect(await screen.findByText('private spoken secret')).toBeTruthy();
  fireEvent.click(
    await screen.findByRole('button', { name: /visual frame 1\.7s/i }),
  );
  expect(screen.queryByText('private screen secret')).toBeNull();
  fireEvent.click(await screen.findByRole('button', { name: 'Read OCR text' }));
  expect(await screen.findByText('private screen secret')).toBeTruthy();
  await userEvent.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(screen.queryByText('Frame reference')).toBeNull();

  const timeline = await screen.findByLabelText('Track timeline');
  mockTimelineRect(timeline);
  const ruler = await screen.findByLabelText('Time ruler');
  fireEvent.pointerDown(ruler, {
    button: 0,
    clientX: 220,
    clientY: 12,
    pointerId: 1,
  });
  fireEvent.pointerMove(timeline, {
    button: 0,
    clientX: 520,
    clientY: 12,
    pointerId: 1,
  });
  fireEvent.pointerUp(timeline, {
    button: 0,
    clientX: 520,
    clientY: 12,
    pointerId: 1,
  });
  fireEvent.click(screen.getAllByRole('button', { name: /^Redact$/i })[0]!);

  await waitFor(() =>
    expect(aiClient.getRecordingActivity).toHaveBeenCalledTimes(2),
  );
  await waitFor(() =>
    expect(aiClient.getRecordingAiProcessingState).toHaveBeenCalledTimes(2),
  );
  await waitFor(() =>
    expect(aiClient.getRecordingVisualObservations).toHaveBeenCalledTimes(2),
  );
  expect(screen.queryByText('private spoken secret')).toBeNull();
  expect(screen.queryByText('private screen secret')).toBeNull();
  await waitFor(() =>
    expect(
      screen.queryByRole('button', {
        name: /Focus changed at 0:02 - Editor - Notes\.md/i,
      }),
    ).toBeNull(),
  );
});

test('RecordingStudioScreen reloads activity after deleting redactions', async () => {
  const redactedDetail = {
    ...detail,
    redactions: [
      {
        createdAtMs: 12_000,
        endOffsetMs: 4_500,
        id: 'redaction-1',
        reason: null,
        recordingId: 'recording-1',
        startOffsetMs: 1_500,
        trackIds: allTimelineLaneIds,
      },
    ],
  } satisfies LibraryRecordingDetail;
  const restoredActivity = {
    captureStates: [],
    events: [
      {
        applicationName: 'Editor',
        id: 'focus-1',
        kind: 'focused-window',
        occurredAtMs: 2_200,
        offsetMs: 1_200,
        recordingId: 'recording-1',
        title: 'Notes.md',
      },
    ],
    intervals: [],
    recordingId: 'recording-1',
    windowSnapshots: [],
  } satisfies RecordingActivity;
  client.getLibraryRecording.mockResolvedValue(redactedDetail);
  client.deleteLibraryRedaction.mockResolvedValue(detail);
  aiClient.getRecordingActivity
    .mockResolvedValueOnce({
      captureStates: [],
      events: [],
      intervals: [],
      recordingId: 'recording-1',
      windowSnapshots: [],
    } satisfies RecordingActivity)
    .mockResolvedValueOnce({
      captureStates: [],
      events: [],
      intervals: [],
      recordingId: 'recording-1',
      windowSnapshots: [],
    } satisfies RecordingActivity)
    .mockResolvedValue(restoredActivity);
  render(<RecordingStudioScreen />);

  fireEvent.click(
    await screen.findByRole('button', {
      name: 'Video redacted range',
    }),
  );
  fireEvent.click(screen.getByRole('button', { name: /^Unredact$/i }));

  await waitFor(() =>
    expect(aiClient.getRecordingActivity).toHaveBeenCalledTimes(3),
  );
  expect(
    await screen.findByRole('button', {
      name: /Focus changed at 0:01 - Editor - Notes\.md/i,
    }),
  ).toBeTruthy();
});

test('RecordingStudioScreen marks mouse and keyboard lanes when input capture was unavailable', async () => {
  aiClient.getRecordingActivity.mockResolvedValue({
    captureStates: [
      {
        id: 'capture-state-1',
        offsetMs: 0,
        reason: 'permission-denied',
        recordingId: 'recording-1',
        source: 'evdev',
        status: 'unavailable',
      },
    ],
    events: [],
    intervals: [],
    recordingId: 'recording-1',
    windowSnapshots: [],
  } satisfies RecordingActivity);

  render(<RecordingStudioScreen />);

  expect(
    await screen.findByLabelText('Mouse activity unavailable'),
  ).toBeTruthy();
  expect(screen.getByLabelText('Keyboard activity unavailable')).toBeTruthy();
});

test('RecordingStudioScreen renders transcript text by transcript segment offsets', async () => {
  aiClient.getRecordingAiProcessingState.mockResolvedValue({
    chunks: [],
    job: {
      createdAtMs: 1,
      errorCode: null,
      id: 'job-1',
      kind: 'ai-processing',
      progress: 1,
      recordingId: 'recording-1',
      status: 'succeeded',
      updatedAtMs: 2,
    },
    recordingId: 'recording-1',
    transcript: {
      recordingId: 'recording-1',
      tracks: [
        {
          segments: [
            {
              confidence: null,
              endOffsetMs: 800,
              id: 'transcript-1',
              model: 'small',
              provider: 'local-whisper-cpp',
              segmentId: 'segment-1',
              startOffsetMs: 500,
              text: 'First words',
              words: [],
            },
            {
              confidence: null,
              endOffsetMs: 1_500,
              id: 'transcript-2',
              model: 'small',
              provider: 'local-whisper-cpp',
              segmentId: 'segment-1',
              startOffsetMs: 1_200,
              text: 'Later words',
              words: [],
            },
          ],
          sourceRole: 'desktop-audio',
          trackId: 'desktop-audio-1',
        },
      ],
    },
    transcriptionTracks: [
      {
        chunks: [
          {
            attemptCount: 1,
            endOffsetMs: 2_000,
            errorCode: null,
            id: 'chunk-1',
            progress: 1,
            result: 'text',
            resultCode: null,
            segmentId: 'segment-1',
            startOffsetMs: 0,
            status: 'succeeded',
            text: 'First words Later words',
            transcriptSegments: [
              {
                confidence: null,
                endOffsetMs: 800,
                id: 'transcript-1',
                model: 'small',
                provider: 'local-whisper-cpp',
                segmentId: 'segment-1',
                startOffsetMs: 500,
                text: 'First words',
                words: [],
              },
              {
                confidence: null,
                endOffsetMs: 1_500,
                id: 'transcript-2',
                model: 'small',
                provider: 'local-whisper-cpp',
                segmentId: 'segment-1',
                startOffsetMs: 1_200,
                text: 'Later words',
                words: [],
              },
            ],
          },
        ],
        description: 'Desktop audio transcription',
        label: 'Transcription',
        libraryTrackId: 'desktop-audio-1',
        sourceRole: 'desktop-audio',
        status: 'complete',
        statusLabel: 'Transcription complete',
        textSegmentCount: 2,
        trackId: 'desktop-audio-1',
        trackKey: 'desktop-audio',
      },
    ],
  });

  render(<RecordingStudioScreen />);

  const firstText = await screen.findByText('First words');
  const secondText = screen.getByText('Later words');
  expect(
    screen.getAllByRole('button', {
      name: /Inspect Desktop audio transcript/i,
    }),
  ).toHaveLength(2);

  const firstBlock = firstText.parentElement;
  const secondBlock = secondText.parentElement;
  expect(firstBlock).toBeTruthy();
  expect(secondBlock).toBeTruthy();
  expect(firstBlock?.style.left).toBe('25%');
  expect(firstBlock?.style.width).toBe('15%');
  expect(secondBlock?.style.left).toBe('60%');
  expect(secondBlock?.style.width).toBe('15%');
  const firstMarker = screen.getByRole('button', {
    name: /Inspect Desktop audio transcript 0:01 to 0:01/i,
  });
  firstMarker.focus();
  expect(await screen.findByRole('tooltip')).toBeTruthy();
  fireEvent.click(firstMarker);
  const inspector = await screen.findByRole('region', {
    name: 'Desktop audio transcript',
  });
  expect(within(inspector).getByText('First words')).toBeTruthy();
  expect(within(inspector).getByText('local-whisper-cpp')).toBeTruthy();
  expect(within(inspector).getByText('segment-1')).toBeTruthy();
  expect(
    screen
      .getByRole('slider', { name: 'Playback position' })
      .getAttribute('aria-valuenow'),
  ).toBe('500');
});

test('RecordingStudioScreen renames the recording from the detail header', async () => {
  render(<RecordingStudioScreen />);

  expect(await screen.findByText('Tracks')).toBeTruthy();
  expect(client.createLibraryPlaybackSession).toHaveBeenCalledTimes(1);

  fireEvent.click(screen.getByRole('button', { name: /rename/i }));
  fireEvent.change(screen.getByLabelText('Title'), {
    target: { value: 'Planning recap' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

  await waitFor(() =>
    expect(client.renameLibraryRecording).toHaveBeenCalledWith({
      recordingId: 'recording-1',
      title: 'Planning recap',
    }),
  );
  expect(
    await screen.findByRole('heading', { name: 'Planning recap' }),
  ).toBeTruthy();
  expect(client.createLibraryPlaybackSession).toHaveBeenCalledTimes(1);
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

test('RecordingStudioScreen maps shift-wheel gestures to horizontal track scrolling', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);

  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  const viewport = trackTimelineViewport(timeline);
  mockTimelineScrollMetrics(viewport);

  expect(dispatchTrackWheel(timeline, { deltaY: 120 }).defaultPrevented).toBe(
    false,
  );
  expect(viewport.scrollLeft).toBe(0);

  expect(
    dispatchTrackWheel(timeline, {
      deltaY: 120,
      shiftKey: true,
    }).defaultPrevented,
  ).toBe(true);

  expect(viewport.scrollLeft).toBe(120);

  expect(
    dispatchTrackWheel(timeline, {
      deltaX: 80,
      deltaY: 400,
      shiftKey: true,
    }).defaultPrevented,
  ).toBe(true);

  expect(viewport.scrollLeft).toBe(200);
  expect(screen.getByText('167%')).toBeTruthy();
});

test('RecordingStudioScreen disables vertical wheel defaults while track zooming', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);

  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  const viewport = trackTimelineViewport(timeline);
  mockTimelineScrollMetrics(viewport);
  const timelineRect = timeline.getBoundingClientRect();

  expect(
    dispatchTrackWheel(timeline, {
      clientX: timelineRect.left + 480,
      ctrlKey: true,
      deltaY: -100,
    }).defaultPrevented,
  ).toBe(true);

  await waitFor(() => expect(screen.getByText('208%')).toBeTruthy());
  expect(viewport.scrollLeft).toBe(0);

  expect(
    dispatchTrackWheel(timeline, {
      ctrlKey: true,
      deltaY: 100,
    }).defaultPrevented,
  ).toBe(true);
});

test('RecordingStudioScreen disables vertical wheel defaults while shift-scrolling tracks', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);

  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  const viewport = trackTimelineViewport(timeline);
  mockTimelineScrollMetrics(viewport);

  expect(
    dispatchTrackWheel(timeline, {
      deltaY: 120,
      shiftKey: true,
    }).defaultPrevented,
  ).toBe(true);

  expect(viewport.scrollLeft).toBe(120);

  expect(
    dispatchTrackWheel(timeline, {
      deltaY: -120,
      shiftKey: true,
    }).defaultPrevented,
  ).toBe(true);

  expect(viewport.scrollLeft).toBe(0);
});

test('RecordingStudioScreen ignores horizontal track wheel gestures without shift', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);

  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  const viewport = trackTimelineViewport(timeline);
  mockTimelineScrollMetrics(viewport);

  expect(
    dispatchTrackWheel(timeline, {
      deltaX: 80,
      deltaY: 400,
    }).defaultPrevented,
  ).toBe(false);

  expect(viewport.scrollLeft).toBe(0);
  expect(screen.getByText('167%')).toBeTruthy();
});

test('RecordingStudioScreen maps shift-wheel line deltas to horizontal track scrolling', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);

  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  const viewport = trackTimelineViewport(timeline);
  mockTimelineScrollMetrics(viewport);

  expect(
    dispatchTrackWheel(timeline, {
      deltaMode: 1,
      deltaY: 3,
      shiftKey: true,
    }).defaultPrevented,
  ).toBe(true);

  expect(viewport.scrollLeft).toBe(48);
});

test('RecordingStudioScreen maps shift-wheel page deltas to horizontal track scrolling', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);

  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  const viewport = trackTimelineViewport(timeline);
  mockTimelineScrollMetrics(viewport);

  expect(
    dispatchTrackWheel(timeline, {
      deltaMode: 2,
      deltaY: 1,
      shiftKey: true,
    }).defaultPrevented,
  ).toBe(true);

  expect(viewport.scrollLeft).toBe(800);
});

test('RecordingStudioScreen clamps shift-wheel horizontal scrolling to the timeline bounds', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);

  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  const viewport = trackTimelineViewport(timeline);
  mockTimelineScrollMetrics(viewport);

  fireEvent.wheel(timeline, {
    deltaY: 3_000,
    shiftKey: true,
  });

  expect(viewport.scrollLeft).toBe(1_600);

  fireEvent.wheel(timeline, {
    deltaY: -3_000,
    shiftKey: true,
  });

  expect(viewport.scrollLeft).toBe(0);
});

test('RecordingStudioScreen keeps ctrl-wheel zoom separate from shift-wheel scrolling', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);

  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  const viewport = trackTimelineViewport(timeline);
  mockTimelineScrollMetrics(viewport);
  const timelineRect = timeline.getBoundingClientRect();

  fireEvent.wheel(timeline, {
    clientX: timelineRect.left + 480,
    ctrlKey: true,
    deltaY: 120,
    shiftKey: true,
  });

  await waitFor(() => expect(screen.getByText('133%')).toBeTruthy());
  expect(viewport.scrollLeft).toBe(0);
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

test('RecordingStudioScreen selects a lane and edits inspector fields', async () => {
  render(<RecordingStudioScreen />);

  const laneButton = await screen.findByRole('button', {
    name: /^Select Desktop audio$/i,
  });
  fireEvent.click(laneButton);

  const trackName = await screen.findByLabelText('Track name');
  expect((trackName as HTMLInputElement).value).toBe('Desktop audio');
  expect(screen.queryByText('recording-1')).toBeNull();
  expect(screen.queryByRole('button', { name: /^Redact$/i })).toBeNull();

  fireEvent.change(trackName, { target: { value: 'System audio' } });
  expect(screen.getByDisplayValue('System audio')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /select track color/i }));
  fireEvent.click(await screen.findByRole('button', { name: /^Rose$/i }));
  expect(
    screen.getByRole('button', { name: /select track color/i }).textContent,
  ).toContain('Rose');
});

test('RecordingStudioScreen renders each selected lane as its own inspector section', async () => {
  render(<RecordingStudioScreen />);

  fireEvent.click(
    await screen.findByRole('button', {
      name: /^Select Video$/i,
    }),
  );
  fireEvent.click(
    await screen.findByRole('button', {
      name: /^Select Desktop audio$/i,
    }),
    { ctrlKey: true },
  );

  const trackNames = await screen.findAllByLabelText('Track name');
  expect(trackNames.map((input) => (input as HTMLInputElement).value)).toEqual([
    'Video',
    'Desktop audio',
  ]);
  expect(screen.queryByText('recording-1')).toBeNull();
  expect(screen.queryByRole('button', { name: /^Redact$/i })).toBeNull();
});

test('RecordingStudioScreen selects a track from the lane body', async () => {
  render(<RecordingStudioScreen />);

  await screen.findByLabelText('Track timeline');
  const laneBody = document.querySelector(
    '[data-lane-content][data-track-id="desktop-audio-1"]',
  );
  expect(laneBody).toBeTruthy();

  fireEvent.click(laneBody as HTMLElement);

  const trackName = await screen.findByLabelText('Track name');
  expect((trackName as HTMLInputElement).value).toBe('Desktop audio');
});

test('RecordingStudioScreen ctrl-click adds a lane from the lane body', async () => {
  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  mockTimelineRect(timeline);
  fireEvent.click(
    await screen.findByRole('button', {
      name: /^Select Video$/i,
    }),
  );
  const laneBody = document.querySelector(
    '[data-lane-content][data-track-id="desktop-audio-1"]',
  );
  expect(laneBody).toBeTruthy();

  fireEvent.pointerDown(laneBody as HTMLElement, {
    button: 0,
    clientX: 420,
    clientY: 112,
    ctrlKey: true,
    pointerId: 1,
  });
  fireEvent.pointerUp(laneBody as HTMLElement, {
    button: 0,
    clientX: 420,
    clientY: 112,
    ctrlKey: true,
    pointerId: 1,
  });
  fireEvent.click(laneBody as HTMLElement, { ctrlKey: true });

  const trackNames = await screen.findAllByLabelText('Track name');
  expect(trackNames.map((input) => (input as HTMLInputElement).value)).toEqual([
    'Video',
    'Desktop audio',
  ]);
});

test('RecordingStudioScreen clears selection on a plain timeline click', async () => {
  render(<RecordingStudioScreen />);

  fireEvent.click(
    await screen.findByRole('button', {
      name: /^Select Desktop audio$/i,
    }),
  );
  expect(await screen.findByLabelText('Track name')).toBeTruthy();

  const timeline = await screen.findByLabelText('Track timeline');
  mockTimelineRect(timeline);
  fireEvent.pointerDown(timeline, {
    button: 0,
    clientX: 420,
    clientY: 112,
    pointerId: 1,
  });
  fireEvent.pointerUp(timeline, {
    button: 0,
    clientX: 420,
    clientY: 112,
    pointerId: 1,
  });

  await waitFor(() =>
    expect(useStudioTimelineStore.getState().selection).toEqual({
      clipIds: [],
      laneIds: [],
      redactionIds: [],
      ranges: [],
      visualFrameIds: [],
    }),
  );
  expect(screen.queryByLabelText('Track name')).toBeNull();
});

test('RecordingStudioScreen selects all lanes from a ruler drag', async () => {
  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  mockTimelineRect(timeline);
  const ruler = await screen.findByLabelText('Time ruler');

  fireEvent.pointerDown(ruler, {
    button: 0,
    clientX: 220,
    clientY: 12,
    pointerId: 1,
  });
  fireEvent.pointerMove(timeline, {
    button: 0,
    clientX: 520,
    clientY: 12,
    pointerId: 1,
  });
  fireEvent.pointerUp(timeline, {
    button: 0,
    clientX: 520,
    clientY: 12,
    pointerId: 1,
  });

  expect(await screen.findByText('5 lanes')).toBeTruthy();
  expect(screen.getByText('Range 1')).toBeTruthy();
  expect(screen.queryByLabelText('Track name')).toBeNull();
  expect(screen.queryByText('recording-1')).toBeNull();
  expect(screen.getAllByLabelText('Selected time range')).toHaveLength(1);
  expect(screen.getAllByRole('button', { name: /^Redact$/i })).toHaveLength(2);
  expect(screen.queryByRole('button', { name: /^Cut$/i })).toBeNull();
});

test('RecordingStudioScreen persists selected ranges as lane redactions', async () => {
  const redactedDetail = {
    ...detail,
    redactions: [
      {
        createdAtMs: 12_000,
        endOffsetMs: 4_500,
        id: 'redaction-1',
        reason: null,
        recordingId: 'recording-1',
        startOffsetMs: 1_500,
        trackIds: ['desktop-audio-1', 'video-0'],
      },
    ],
  } satisfies LibraryRecordingDetail;
  client.createLibraryRedactions.mockResolvedValue(redactedDetail);
  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  mockTimelineRect(timeline);
  const ruler = await screen.findByLabelText('Time ruler');
  fireEvent.pointerDown(ruler, {
    button: 0,
    clientX: 220,
    clientY: 12,
    pointerId: 1,
  });
  fireEvent.pointerMove(timeline, {
    button: 0,
    clientX: 520,
    clientY: 12,
    pointerId: 1,
  });
  fireEvent.pointerUp(timeline, {
    button: 0,
    clientX: 520,
    clientY: 12,
    pointerId: 1,
  });

  fireEvent.click(screen.getAllByRole('button', { name: /^Redact$/i })[0]!);

  await waitFor(() =>
    expect(client.createLibraryRedactions).toHaveBeenCalledWith({
      ranges: [
        {
          endOffsetMs: expect.any(Number),
          startOffsetMs: expect.any(Number),
          trackIds: allTimelineLaneIds,
        },
      ],
      recordingId: 'recording-1',
    }),
  );
  expect(await screen.findAllByText('Redacted')).toHaveLength(2);
  expect(screen.queryByLabelText('Selected time range')).toBeNull();
});

test('RecordingStudioScreen renders timeline redaction masks as opaque controls', async () => {
  client.getLibraryRecording.mockResolvedValue({
    ...detail,
    redactions: [
      {
        createdAtMs: 12_000,
        endOffsetMs: 4_500,
        id: 'redaction-1',
        reason: null,
        recordingId: 'recording-1',
        startOffsetMs: 1_500,
        trackIds: ['video-0'],
      },
    ],
  } satisfies LibraryRecordingDetail);

  render(<RecordingStudioScreen />);

  const redactionMask = await screen.findByRole('button', {
    name: 'Video redacted range',
  });
  expect(redactionMask.getAttribute('aria-pressed')).toBe('false');
  expect(redactionMask.textContent).toMatch(/Redacted\d+ to \d+ ms/);
});

test('RecordingStudioScreen removes a selected redaction mask', async () => {
  const redactedDetail = {
    ...detail,
    redactions: [
      {
        createdAtMs: 12_000,
        endOffsetMs: 4_500,
        id: 'redaction-1',
        reason: null,
        recordingId: 'recording-1',
        startOffsetMs: 1_500,
        trackIds: ['video-0'],
      },
    ],
  } satisfies LibraryRecordingDetail;
  client.getLibraryRecording.mockResolvedValue(redactedDetail);
  client.unredactLibraryRedactions.mockResolvedValue({
    ...detail,
    redactions: [
      {
        createdAtMs: 12_001,
        endOffsetMs: 1_500,
        id: 'redaction-left',
        reason: null,
        recordingId: 'recording-1',
        startOffsetMs: 1_000,
        trackIds: ['desktop-audio-1', 'video-0'],
      },
      {
        createdAtMs: 12_001,
        endOffsetMs: 4_500,
        id: 'redaction-right',
        reason: null,
        recordingId: 'recording-1',
        startOffsetMs: 3_000,
        trackIds: ['desktop-audio-1', 'video-0'],
      },
    ],
  } satisfies LibraryRecordingDetail);
  render(<RecordingStudioScreen />);

  fireEvent.click(
    await screen.findByRole('button', {
      name: 'Video redacted range',
    }),
  );
  expect(
    await screen.findByRole('button', { name: /^Unredact$/i }),
  ).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /^Unredact$/i }));

  await waitFor(() =>
    expect(client.deleteLibraryRedaction).toHaveBeenCalledWith({
      recordingId: 'recording-1',
      redactionId: 'redaction-1',
    }),
  );
});

test('RecordingStudioScreen removes a redaction overlapped by a time selection', async () => {
  const redactedDetail = {
    ...detail,
    redactions: [
      {
        createdAtMs: 12_000,
        endOffsetMs: 4_500,
        id: 'redaction-1',
        reason: null,
        recordingId: 'recording-1',
        startOffsetMs: 1_500,
        trackIds: ['desktop-audio-1', 'video-0'],
      },
    ],
  } satisfies LibraryRecordingDetail;
  client.getLibraryRecording.mockResolvedValue(redactedDetail);
  client.deleteLibraryRedaction.mockResolvedValue({
    ...detail,
    redactions: [],
  } satisfies LibraryRecordingDetail);
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype, 'scrollWidth', 'get').mockReturnValue(2_400);
  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  mockTimelineRect(timeline);
  await waitFor(() => expect(timeline.style.width).toBe('800px'));
  const ruler = await screen.findByLabelText('Time ruler');

  fireEvent.pointerDown(ruler, {
    button: 0,
    clientX: 220,
    clientY: 12,
    pointerId: 1,
  });
  fireEvent.pointerMove(timeline, {
    button: 0,
    clientX: 520,
    clientY: 12,
    pointerId: 1,
  });
  fireEvent.pointerUp(timeline, {
    button: 0,
    clientX: 520,
    clientY: 12,
    pointerId: 1,
  });

  expect(await screen.findByText('5 lanes')).toBeTruthy();
  expect(screen.getAllByRole('button', { name: /^Unredact$/i })).toHaveLength(
    2,
  );

  fireEvent.click(screen.getAllByRole('button', { name: /^Unredact$/i })[0]!);

  await waitFor(() =>
    expect(client.unredactLibraryRedactions).toHaveBeenCalledWith({
      ranges: [
        {
          endOffsetMs: expect.any(Number),
          startOffsetMs: expect.any(Number),
          trackIds: allTimelineLaneIds,
        },
      ],
      recordingId: 'recording-1',
    }),
  );
  expect(client.deleteLibraryRedaction).not.toHaveBeenCalled();
  expect(screen.queryByLabelText('Selected time range')).toBeNull();
});

test('RecordingStudioScreen ctrl-drag creates one additive range despite many moves', async () => {
  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  mockTimelineRect(timeline);
  const ruler = await screen.findByLabelText('Time ruler');

  dragTimelineRange({
    clientY: 12,
    ctrlKey: true,
    pointerId: 1,
    target: ruler,
    timeline,
    xValues: [220, 260, 300, 360, 420, 520],
  });

  await waitFor(() =>
    expect(useStudioTimelineStore.getState().selection.ranges).toHaveLength(1),
  );
  expect(screen.getAllByLabelText('Selected time range')).toHaveLength(1);
});

test('RecordingStudioScreen ctrl-drag adds one range to an existing range', async () => {
  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  mockTimelineRect(timeline);
  const ruler = await screen.findByLabelText('Time ruler');
  useStudioTimelineStore.getState().setSelection({
    clipIds: [],
    laneIds: [],
    redactionIds: [],
    ranges: [
      {
        endMs: 2_000,
        id: 'seed-range',
        laneIds: allTimelineLaneIds,
        startMs: 1_000,
      },
    ],
    visualFrameIds: [],
  });
  await waitFor(() =>
    expect(screen.getAllByLabelText('Selected time range')).toHaveLength(1),
  );

  dragTimelineRange({
    clientY: 12,
    ctrlKey: true,
    pointerId: 2,
    target: ruler,
    timeline,
    xValues: [
      timelineClientXForMs(timeline, 3_000),
      timelineClientXForMs(timeline, 3_400),
      timelineClientXForMs(timeline, 3_800),
      timelineClientXForMs(timeline, 4_000),
    ],
  });

  await waitFor(() =>
    expect(useStudioTimelineStore.getState().selection.ranges).toHaveLength(2),
  );
  expect(screen.getAllByLabelText('Selected time range')).toHaveLength(2);
});

test('RecordingStudioScreen ctrl-shift-drag subtracts from an existing range', async () => {
  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  mockTimelineRect(timeline);
  const ruler = await screen.findByLabelText('Time ruler');
  useStudioTimelineStore.getState().setSelection({
    clipIds: [],
    laneIds: [],
    redactionIds: [],
    ranges: [
      {
        endMs: 7_000,
        id: 'seed-range',
        laneIds: ['video-0', 'desktop-audio-1'],
        startMs: 0,
      },
    ],
    visualFrameIds: [],
  });
  await waitFor(() =>
    expect(screen.getAllByLabelText('Selected time range')).toHaveLength(1),
  );

  dragTimelineRange({
    clientY: 12,
    ctrlKey: true,
    pointerId: 3,
    shiftKey: true,
    target: ruler,
    timeline,
    xValues: [
      timelineClientXForMs(timeline, 3_000),
      timelineClientXForMs(timeline, 3_250),
      timelineClientXForMs(timeline, 3_500),
      timelineClientXForMs(timeline, 3_750),
      timelineClientXForMs(timeline, 4_000),
    ],
  });

  await waitFor(() => {
    const [range] = useStudioTimelineStore.getState().selection.ranges;
    expect(range).toBeDefined();
  });
  const remainingRange = useStudioTimelineStore.getState().selection.ranges[0]!;
  expect(remainingRange).toMatchObject({
    laneIds: ['desktop-audio-1', 'video-0'],
  });
  expect(
    screen.getAllByLabelText('Selected time range').length,
  ).toBeGreaterThan(0);
});

test('RecordingStudioScreen ctrl-drag extends an overlapping existing range', async () => {
  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  mockTimelineRect(timeline);
  const ruler = await screen.findByLabelText('Time ruler');
  useStudioTimelineStore.getState().setSelection({
    clipIds: [],
    laneIds: [],
    redactionIds: [],
    ranges: [
      {
        endMs: 7_000,
        id: 'seed-range',
        laneIds: allTimelineLaneIds,
        startMs: 6_000,
      },
    ],
    visualFrameIds: [],
  });
  await waitFor(() =>
    expect(screen.getAllByLabelText('Selected time range')).toHaveLength(1),
  );
  const firstRange = useStudioTimelineStore.getState().selection.ranges[0]!;

  dragTimelineRange({
    clientY: 12,
    ctrlKey: true,
    pointerId: 2,
    target: ruler,
    timeline,
    xValues: [
      timelineClientXForMs(timeline, 5_500),
      timelineClientXForMs(timeline, 6_500),
    ],
  });

  await waitFor(() =>
    expect(useStudioTimelineStore.getState().selection.ranges).toHaveLength(1),
  );
  const mergedRange = useStudioTimelineStore.getState().selection.ranges[0]!;
  expect(mergedRange).toMatchObject({
    endMs: firstRange.endMs,
    laneIds: allTimelineLaneIds,
    startMs: 5_500,
  });
  expect(screen.getAllByLabelText('Selected time range')).toHaveLength(1);
});

test('RecordingStudioScreen marquee-selects only intersected lane ranges', async () => {
  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  mockTimelineRect(timeline);
  const firstLaneDragStartY = TRACK_RULER_HEIGHT_PX + 16;
  const firstLaneDragEndY = TRACK_RULER_HEIGHT_PX + TRACK_LANE_HEIGHT_PX - 16;

  fireEvent.pointerDown(timeline, {
    button: 0,
    clientX: 220,
    clientY: firstLaneDragStartY,
    pointerId: 1,
  });
  fireEvent.pointerMove(timeline, {
    button: 0,
    clientX: 520,
    clientY: firstLaneDragEndY,
    pointerId: 1,
  });
  fireEvent.pointerUp(timeline, {
    button: 0,
    clientX: 520,
    clientY: firstLaneDragEndY,
    pointerId: 1,
  });

  expect(await screen.findByText('1 lane')).toBeTruthy();
  expect(screen.getByText('Range 1')).toBeTruthy();
  expect(screen.queryByLabelText('Track name')).toBeNull();
  expect(screen.queryByText('recording-1')).toBeNull();
});

test('RecordingStudioScreen selects and renames a timeline clip', async () => {
  render(<RecordingStudioScreen />);

  fireEvent.click(
    await screen.findByRole('button', {
      name: /^Desktop audio Segment 1$/i,
    }),
  );

  const clipTitle = await screen.findByLabelText('Clip title');
  expect((clipTitle as HTMLInputElement).value).toBe('Segment 1');
  expect(screen.queryByLabelText('Track name')).toBeNull();
  expect(screen.queryByText('recording-1')).toBeNull();

  fireEvent.change(clipTitle, { target: { value: 'Intro clip' } });
  expect(screen.getByDisplayValue('Intro clip')).toBeTruthy();
  expect(
    screen.getByRole('button', { name: /^Desktop audio Intro clip$/i }),
  ).toBeTruthy();
});

test('RecordingStudioScreen exposes compact lane reorder handles', async () => {
  render(<RecordingStudioScreen />);

  expect(
    await screen.findByRole('button', { name: /^Reorder Video$/i }),
  ).toBeTruthy();
  expect(
    screen.getByRole('button', { name: /^Reorder Desktop audio$/i }),
  ).toBeTruthy();
  expect(
    screen.queryByRole('button', { name: /^Move Video down$/i }),
  ).toBeNull();
  expect(
    screen
      .getAllByRole('button', { name: /^Reorder /i })
      .map((button) => button.getAttribute('aria-label')),
  ).toEqual([
    'Reorder Video',
    'Reorder Desktop audio',
    'Reorder Mouse',
    'Reorder Keyboard',
    'Reorder Activity',
  ]);
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

test('RecordingStudioScreen fullscreen targets the player surface instead of the raw video', async () => {
  const fullscreenPrototype = HTMLElement.prototype as Partial<HTMLElement>;
  const originalRequestFullscreen = fullscreenPrototype.requestFullscreen;
  const requestFullscreen = vi.fn(() => Promise.resolve());
  Object.defineProperty(fullscreenPrototype, 'requestFullscreen', {
    configurable: true,
    value: requestFullscreen,
  });

  try {
    const { container } = render(<RecordingStudioScreen />);

    const video = await waitFor(() => {
      const element = container.querySelector('video');
      expect(element).toBeTruthy();
      return element!;
    });

    fireEvent.click(screen.getByRole('button', { name: /^fullscreen$/i }));

    expect(requestFullscreen).toHaveBeenCalledTimes(1);
    const fullscreenTarget = requestFullscreen.mock.contexts[0] as HTMLElement;
    expect(fullscreenTarget).not.toBe(video);
    expect(fullscreenTarget.querySelector('video')).toBe(video);
  } finally {
    if (originalRequestFullscreen) {
      Object.defineProperty(fullscreenPrototype, 'requestFullscreen', {
        configurable: true,
        value: originalRequestFullscreen,
      });
    } else {
      delete fullscreenPrototype.requestFullscreen;
    }
  }
});

test('RecordingStudioScreen masks and mutes redactions from media time before timeupdate', async () => {
  client.getLibraryRecording.mockResolvedValue({
    ...detail,
    redactions: [
      {
        createdAtMs: 12_000,
        endOffsetMs: 1_500,
        id: 'redaction-1',
        reason: null,
        recordingId: 'recording-1',
        startOffsetMs: 1_000,
        trackIds: ['desktop-audio-1', 'video-0'],
      },
    ],
  } satisfies LibraryRecordingDetail);

  const { container } = render(<RecordingStudioScreen />);

  const video = await waitFor(() => {
    const element = container.querySelector('video');
    expect(element).toBeTruthy();
    return element!;
  });
  const audio = await waitFor(() => {
    const element = container.querySelector('audio');
    expect(element).toBeTruthy();
    return element!;
  });

  video.currentTime = 0.95;
  fireEvent.play(video);

  await waitFor(() => {
    expect(within(video.parentElement!).getByText('Redacted')).toBeTruthy();
    expect(
      within(video.parentElement!).getByRole('button', {
        name: /skip redacted part/i,
      }),
    ).toBeTruthy();
    expect(audio.muted).toBe(true);
  });

  fireEvent.click(
    within(video.parentElement!).getByRole('button', {
      name: /skip redacted part/i,
    }),
  );

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}?startMs=1500&request=1`,
    ),
  );

  const skippedVideo = container.querySelector('video')!;
  skippedVideo.currentTime = 1.5;
  fireEvent.timeUpdate(skippedVideo);

  await waitFor(() => {
    expect(
      within(skippedVideo.parentElement!).queryByText('Redacted'),
    ).toBeNull();
    expect(audio.muted).toBe(false);
  });
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

test('RecordingStudioScreen keeps the desktop track workspace tall enough to edit', async () => {
  layoutStylesheet = document.createElement('link');
  layoutStylesheet.rel = 'stylesheet';
  layoutStylesheet.href = '/src/index.css';
  const stylesheetLoaded = new Promise<void>((resolve, reject) => {
    if (!layoutStylesheet) {
      reject(new Error('Failed to create the workspace stylesheet link.'));
      return;
    }
    layoutStylesheet.addEventListener('load', () => resolve(), { once: true });
    layoutStylesheet.addEventListener(
      'error',
      () => reject(new Error('Could not load the workspace stylesheet.')),
      { once: true },
    );
  });
  document.head.append(layoutStylesheet);
  await stylesheetLoaded;
  await setTestViewport(1440, 960);
  const { container } = render(
    <div className="h-dvh w-full">
      <RecordingStudioScreen />
    </div>,
  );

  const timeline = await screen.findByRole('group', { name: 'Track timeline' });
  const panelGroup = container.querySelector(
    '[data-slot="resizable-panel-group"]',
  );
  if (!panelGroup) {
    throw new Error('Expected the resizable studio workspace.');
  }

  await waitFor(() =>
    expect(panelGroup.getBoundingClientRect().height).toBeGreaterThan(700),
  );
  expect(timeline.getBoundingClientRect().height).toBeGreaterThan(200);
});

test('RecordingStudioScreen retains playback time while switching compact panels', async () => {
  await setTestViewport(620, 960);
  const { container } = render(
    <div className="h-dvh w-full">
      <RecordingStudioScreen />
    </div>,
  );

  await screen.findByRole('tablist', { name: 'Recording studio panels' });
  const video = container.querySelector('video');
  if (!video) {
    throw new Error('Expected the recording player to mount.');
  }

  Object.defineProperty(video, 'currentTime', {
    configurable: true,
    value: 1.25,
  });
  fireEvent.timeUpdate(video);
  await selectStudioPanel('Inspector');

  expect(container.querySelector('video')).toBe(video);
  expect(video.currentTime).toBe(1.25);
  expect(
    screen
      .getByRole('tab', { name: 'Inspector' })
      .getAttribute('aria-selected'),
  ).toBe('true');

  await selectStudioPanel('Timeline');

  expect(container.querySelector('video')).toBe(video);
  expect(video.currentTime).toBe(1.25);
  expect(
    await screen.findByRole('group', { name: 'Track timeline' }),
  ).toBeTruthy();
});

test('RecordingStudioScreen commits a lower mouse-lane seek before playback', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);
  aiClient.getRecordingActivity.mockResolvedValue({
    captureStates: [],
    events: [],
    intervals: [
      {
        endOffsetMs: 6_000,
        id: 'mouse-interval',
        jobId: 'activity-job',
        recordingId: 'recording-1',
        score: 1,
        sourceKind: 'mouse',
        startOffsetMs: 5_000,
        state: 'active',
      },
    ],
    recordingId: 'recording-1',
    windowSnapshots: [],
  });

  const { container } = render(<RecordingStudioScreen />);
  const timeline = await screen.findByLabelText('Track timeline');
  mockTimelineRect(timeline);
  await mockPlaybackStartAt(timeline, 12_000);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${longPlaybackAssets.segments[0]!.videoUrl}?startMs=12000&request=1`,
    ),
  );

  const playbackPosition = screen.getByRole('slider', {
    name: /playback position/i,
  });
  fireEvent.click(screen.getByRole('button', { name: /^play$/i }));
  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalled(),
  );
  const video = container.querySelector('video');
  if (!video) {
    throw new Error('Expected the playback video element.');
  }
  video.currentTime = 0.4;
  fireEvent.timeUpdate(video);

  await waitFor(() =>
    expect(
      Number(playbackPosition.getAttribute('aria-valuenow')),
    ).toBeGreaterThan(12_000),
  );
});

test('RecordingStudioScreen inspects accessible activity spans without exposing input contents', async () => {
  aiClient.getRecordingActivity.mockResolvedValue({
    captureStates: [],
    events: [],
    intervals: [
      {
        endOffsetMs: 6_000,
        id: 'mouse-interval',
        jobId: 'activity-job',
        recordingId: 'recording-1',
        score: 0.75,
        sourceKind: 'mouse',
        startOffsetMs: 5_000,
        state: 'active',
      },
    ],
    recordingId: 'recording-1',
    windowSnapshots: [],
  });

  const { container } = render(<RecordingStudioScreen />);
  const interval = await screen.findByRole('button', {
    name: /mouse activity.*0:05.*0:06/i,
  });
  interval.focus();
  expect(await screen.findByRole('tooltip')).toBeTruthy();
  fireEvent.click(interval);

  expect(await screen.findByText('Activity source')).toBeTruthy();
  const inspector = screen.getByRole('region', {
    name: 'Mouse activity interval',
  });
  expect(within(inspector).getByText('Mouse')).toBeTruthy();
  expect(within(inspector).getByText('0:05')).toBeTruthy();
  expect(within(inspector).getByText('0:06')).toBeTruthy();
  expect(within(inspector).getByText('0.75')).toBeTruthy();
  expect(container.textContent).not.toContain('keystroke');
});

test('RecordingStudioScreen toggles playback with Space outside controls', async () => {
  const { container } = render(<RecordingStudioScreen />);
  await screen.findByText('Tracks');
  const video = container.querySelector('video');
  if (!video) {
    throw new Error('Expected the playback video element.');
  }

  fireEvent.keyDown(document.body, { code: 'Space', key: ' ' });
  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalledTimes(2),
  );
  expect(screen.getByRole('button', { name: /^pause$/i })).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /^Select Video$/i }));
  const titleInput = screen.getByRole('textbox', { name: /track name/i });
  titleInput.focus();
  fireEvent.keyDown(titleInput, { code: 'Space', key: ' ' });
  expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalledTimes(2);
  expect((titleInput as HTMLInputElement).value).toBe('Video');
});

test('RecordingStudioScreen shows scoped redactions on the player seekbar and skips long masked video only while playing', async () => {
  // Media playback is mocked in this suite; missing IPC fixture URLs must not
  // deliver a real decoder error into the countdown scenario.
  const suppressMockMediaError = (event: Event) => {
    if (event.target instanceof HTMLMediaElement)
      event.stopImmediatePropagation();
  };
  window.addEventListener('error', suppressMockMediaError, true);
  const redactedDetail = {
    ...longDetail,
    redactions: [
      {
        createdAtMs: 1,
        endOffsetMs: 14_000,
        id: 'video-cut',
        reason: null,
        recordingId: longDetail.id,
        startOffsetMs: 6_000,
        trackIds: ['video-0', 'desktop-audio-1'],
      },
      {
        createdAtMs: 1,
        endOffsetMs: 22_000,
        id: 'audio-cut',
        reason: null,
        recordingId: longDetail.id,
        startOffsetMs: 20_000,
        trackIds: ['desktop-audio-1'],
      },
    ],
  } satisfies LibraryRecordingDetail;
  client.getLibraryRecording.mockResolvedValue(redactedDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);
  const { container, unmount } = render(<RecordingStudioScreen />);
  const seekbar = await screen.findByRole('slider', {
    name: 'Playback position',
  });
  expect(
    within(seekbar).getByLabelText(/Redacted video and audio.*0:06.*0:14/i),
  ).toBeTruthy();
  expect(
    within(seekbar).getByLabelText(/Redacted audio.*0:20.*0:22/i),
  ).toBeTruthy();
  const video = container.querySelector('video');
  if (!video) throw new Error('Playback video missing');
  video.currentTime = 6.2;
  fireEvent.timeUpdate(video);
  expect(
    await screen.findByRole('group', { name: 'Redacted video' }),
  ).toBeTruthy();
  expect(screen.queryByText(/Skipping in/)).toBeNull();
  vi.useFakeTimers({
    toFake: [
      'setTimeout',
      'clearTimeout',
      'setInterval',
      'clearInterval',
      'Date',
    ],
  });
  fireEvent.play(video);
  await vi.advanceTimersByTimeAsync(0);
  expect(screen.getByText('Skipping in 3 seconds')).toBeTruthy();
  await act(() => vi.advanceTimersByTimeAsync(1_100));
  expect(screen.getByText('Skipping in 2 seconds')).toBeTruthy();
  await act(() => vi.advanceTimersByTimeAsync(1_900));
  expect(container.querySelector('video')?.getAttribute('src')).toContain(
    'startMs=14000',
  );
  expect(screen.queryByText(/Skipping in/)).toBeNull();
  expect(seekbar.getAttribute('aria-valuenow')).toBe('14000');
  unmount();
  window.removeEventListener('error', suppressMockMediaError, true);
  await vi.advanceTimersByTimeAsync(4_000);
});

test('RecordingStudioScreen exposes activity intervals by source and inspects their provenance', async () => {
  aiClient.getRecordingActivity.mockResolvedValue({
    captureStates: [],
    events: [],
    intervals: [
      {
        endOffsetMs: 6_000,
        id: 'keyboard-interval',
        jobId: 'activity-job',
        recordingId: 'recording-1',
        score: 0.75,
        sourceKind: 'keyboard',
        startOffsetMs: 5_000,
        state: 'active',
      },
    ],
    recordingId: 'recording-1',
    windowSnapshots: [],
  });

  render(<RecordingStudioScreen />);
  const span = await screen.findByRole('button', {
    name: /keyboard activity.*0:05.*0:06/i,
  });
  span.focus();
  expect(await screen.findByRole('tooltip')).toBeTruthy();
  fireEvent.click(span);

  expect(await screen.findByText('Activity source')).toBeTruthy();
  const inspector = screen.getByRole('region', {
    name: 'Keyboard activity interval',
  });
  expect(within(inspector).getByText('Keyboard')).toBeTruthy();
  expect(within(inspector).getByText('0:05')).toBeTruthy();
  expect(within(inspector).getByText('0:06')).toBeTruthy();
  expect(within(inspector).getByText('0.75')).toBeTruthy();
  expect(document.body.textContent).not.toContain('keystroke');
});

test('RecordingStudioScreen toggles playback from focused Space and ignores editable targets', async () => {
  const { container } = render(<RecordingStudioScreen />);
  await screen.findByText('Tracks');
  const video = container.querySelector('video');
  if (!video) {
    throw new Error('Expected the playback video element.');
  }

  fireEvent.keyDown(document.body, { code: 'Space', key: ' ' });
  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalledTimes(2),
  );
  expect(screen.getByRole('button', { name: /^pause$/i })).toBeTruthy();

  // The native button owns Space; the global shortcut must not also toggle.
  Object.defineProperty(video, 'paused', { configurable: true, value: false });
  const pauseButton = screen.getByRole('button', { name: /^Pause$/i });
  pauseButton.focus();
  await userEvent.keyboard(' ');
  expect(screen.getByRole('button', { name: /^Play$/i })).toBeTruthy();
  expect(window.HTMLMediaElement.prototype.pause).toHaveBeenCalledTimes(2);
  expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalledTimes(2);
});

test('RecordingStudioScreen distinguishes desktop no-speech from microphone text and clears a cut selected transcript', async () => {
  const microphoneDetail = {
    ...detail,
    tracks: [
      ...detail.tracks,
      {
        codec: 'opus',
        description: 'Microphone recording',
        id: 'microphone-2',
        kind: 'microphone',
        label: 'Microphone',
        streamIndex: 2,
      },
    ],
  } satisfies LibraryRecordingDetail;
  const transcripts = [500, 1_200, 5_400].map((startOffsetMs, index) => ({
    confidence: 0.9,
    endOffsetMs: startOffsetMs + 300,
    id: `mic-text-${index}`,
    model: 'small',
    provider: 'local-whisper-cpp',
    segmentId: index === 2 ? 'segment-2' : 'segment-1',
    startOffsetMs,
    text: `Synthetic mic phrase ${index + 1}`,
    words: [],
  }));
  const chunk = {
    attemptCount: 1,
    endOffsetMs: 7_000,
    errorCode: null,
    id: 'mic-chunk',
    progress: 1,
    result: 'text',
    resultCode: null,
    segmentId: 'segment-1',
    startOffsetMs: 0,
    status: 'succeeded',
    text: transcripts.map((segment) => segment.text).join(' '),
    transcriptSegments: transcripts,
  } as const;
  aiClient.getRecordingAiProcessingState.mockResolvedValue({
    chunks: [],
    job: null,
    recordingId: detail.id,
    transcript: {
      recordingId: detail.id,
      tracks: [
        {
          sourceRole: 'microphone',
          trackId: 'persisted-mic-uuid',
          segments: transcripts,
        },
      ],
    },
    transcriptionTracks: [
      {
        chunks: [
          {
            ...chunk,
            id: 'desktop-chunk',
            result: 'no-speech',
            text: null,
            transcriptSegments: [],
          },
        ],
        description: 'Desktop transcription',
        label: 'Desktop audio',
        libraryTrackId: 'desktop-audio-1',
        sourceRole: 'desktop-audio',
        status: 'no-speech',
        statusLabel: 'No speech detected',
        textSegmentCount: 0,
        trackId: 'persisted-desktop-uuid',
        trackKey: 'desktop-audio',
      },
      {
        chunks: [{ ...chunk, transcriptSegments: transcripts }],
        description: 'Microphone transcription',
        label: 'Microphone',
        libraryTrackId: 'microphone-2',
        sourceRole: 'microphone',
        status: 'complete',
        statusLabel: 'Transcription complete',
        textSegmentCount: 3,
        trackId: 'persisted-mic-uuid',
        trackKey: 'microphone',
      },
    ],
  } satisfies RecordingAiProcessingState);
  client.getLibraryRecording.mockResolvedValue(microphoneDetail);
  const { container } = render(<RecordingStudioScreen />);
  const summary = await screen.findByRole('region', {
    name: 'Audio transcription',
  });
  expect(within(summary).getByText('No speech detected')).toBeTruthy();
  expect(within(summary).getByText('Transcription complete')).toBeTruthy();
  const marker = await screen.findByRole('button', {
    name: /Inspect Microphone transcript 0:01 to 0:01/i,
  });
  fireEvent.click(marker);
  const selected = await screen.findByRole('region', {
    name: 'Microphone transcript',
  });
  expect(within(selected).getByText('Synthetic mic phrase 1')).toBeTruthy();
  expect(within(selected).getByText('local-whisper-cpp')).toBeTruthy();
  expect(within(selected).getByText('microphone-2')).toBeTruthy();
  expect(screen.queryByText('No speech detected')).toBeNull();
  fireEvent.click(
    screen.getByRole('button', { name: /^Select Desktop audio$/i }),
  );
  expect(await screen.findByText('No speech detected')).toBeTruthy();
  expect(
    screen.queryByRole('region', { name: 'Microphone transcript' }),
  ).toBeNull();
  fireEvent.click(marker);
  expect(
    await screen.findByRole('region', { name: 'Microphone transcript' }),
  ).toBeTruthy();
  client.getLibraryRecording.mockResolvedValue({
    ...microphoneDetail,
    redactions: [
      {
        createdAtMs: 1,
        endOffsetMs: 900,
        id: 'cut-mic',
        reason: null,
        recordingId: detail.id,
        startOffsetMs: 400,
        trackIds: ['microphone-2'],
      },
    ],
  });
  fireEvent.click(screen.getByRole('button', { name: /^Refresh$/i }));
  await waitFor(() =>
    expect(container.textContent).not.toContain('Synthetic mic phrase 1'),
  );
  expect(
    screen.queryByRole('region', { name: 'Microphone transcript' }),
  ).toBeNull();
});

test('RecordingStudioScreen exposes a pre-frame OCR failure from the real video source and retries its scoped job', async () => {
  const job: ProcessingJob = {
    createdAtMs: 1,
    errorCode: 'ai_ocr_vision_unconfigured',
    id: 'pre-frame-ocr-job',
    kind: 'ocr',
    progress: 0,
    recordingId: detail.id,
    status: 'failed',
    updatedAtMs: 2,
  };
  ocrClient.getOcrDocument.mockResolvedValue({
    document: null,
    job,
    recordingId: detail.id,
    sourceTrackId: 'video-0',
  });
  render(<RecordingStudioScreen />);
  await screen.findByText('Tracks');
  expect(screen.queryByRole('button', { name: /visual frame/i })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Select Video' }));
  expect(
    await screen.findByText(/OCR processing failed for this source/),
  ).toBeTruthy();
  expect(ocrClient.getOcrDocument).toHaveBeenCalledWith({
    recordingId: detail.id,
    sourceTrackId: 'video-0',
    timestampMs: 0,
    documentId: null,
  });
  ocrClient.getOcrDocument.mockResolvedValue({
    document: null,
    job: { ...job, status: 'queued', errorCode: null },
    recordingId: detail.id,
    sourceTrackId: 'video-0',
  });
  fireEvent.click(screen.getByRole('button', { name: 'Retry processing' }));
  await waitFor(() =>
    expect(aiClient.retryProcessingJob).toHaveBeenCalledExactlyOnceWith({
      jobId: job.id,
    }),
  );
  expect(await screen.findByText(/OCR processing is queued/)).toBeTruthy();
  expect(ocrClient.reprocessRecordingOcr).not.toHaveBeenCalled();
  expect(screen.queryByLabelText('Recognized text')).toBeNull();
});

test.each([false, true])(
  'RecordingStudioScreen preserves every legacy OCR sample even when text is identical and coarse progress exists: %s',
  async (coarseProgress) => {
    const sampledDetail = {
      ...longDetail,
      durationMs: 38_000,
      segments: [{ ...longDetail.segments[0]!, durationMs: 38_000 }],
    } satisfies LibraryRecordingDetail;
    client.getLibraryRecording.mockResolvedValue(sampledDetail);
    client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);
    const frames = [0, 5_000, 10_000].map((frameTimestampMs) => ({
      id: `sample-${frameTimestampMs}`,
      recordingId: detail.id,
      segmentId: 'segment-long',
      sourceTrackId: 'video-0',
      frameTimestampMs,
      sampleEndMs: frameTimestampMs + 1,
      sourceFrameRef: `opaque-sample-${frameTimestampMs}`,
      status: 'succeeded' as const,
      errorCode: null,
      documentId: `sample-${frameTimestampMs}`,
      ocrOutputCount: 1,
      textEmpty: false,
    }));
    ocrClient.getOcrTimelineFrames.mockImplementation(
      (input: { cursor?: { id: string; timestampMs: number } | null }) =>
        Promise.resolve({
          recordingId: detail.id,
          frames:
            coarseProgress && !input.cursor
              ? frames.slice(0, 2)
              : coarseProgress
                ? frames.slice(2)
                : frames,
          nextCursor:
            coarseProgress && !input.cursor
              ? { id: frames[1]!.id, timestampMs: 5_000 }
              : null,
        } satisfies OcrTimelineFramePage),
    );
    aiClient.getRecordingAiProcessingState.mockResolvedValue({
      chunks: [],
      job: null,
      recordingId: detail.id,
      transcript: { recordingId: detail.id, tracks: [] },
      visualFrames: coarseProgress
        ? [
            {
              chunkId: 'coarse-chunk',
              createdAtMs: 0,
              errorCode: null,
              frameTimestampMs: 0,
              id: 'coarse-progress',
              jobId: 'old-job',
              outputCount: 500,
              progress: 1,
              recordingId: detail.id,
              region: { x: 0, y: 0, width: 1920, height: 1080 },
              segmentId: 'segment-long',
              sourceFrameRef: 'opaque-coarse-progress',
              sourceKind: 'ocr',
              status: 'succeeded',
              updatedAtMs: 1,
            },
          ]
        : [],
    } satisfies RecordingAiProcessingState);
    ocrClient.getOcrDocument.mockImplementation(
      (input: { timestampMs: number; documentId?: string | null }) =>
        Promise.resolve({
          document: {
            ...legacyOcrDocument(
              'Identical literal meeting text',
              input.timestampMs,
            ),
            id: input.documentId ?? `sample-${input.timestampMs}`,
            contentId: 'shared-image-content',
            segmentId: 'segment-long',
          },
          job: null,
          recordingId: detail.id,
          sourceTrackId: 'video-0',
        }),
    );
    render(<RecordingStudioScreen />);
    await waitFor(() =>
      expect(
        screen.getAllByRole('button', { name: /Visual frame/i }),
      ).toHaveLength(3),
    );
    expect(aiClient.getRecordingVisualObservations).toHaveBeenCalledWith({
      recordingId: detail.id,
      includeOcrRegions: false,
    });
    if (coarseProgress)
      expect(ocrClient.getOcrTimelineFrames).toHaveBeenCalledWith({
        recordingId: detail.id,
        cursor: { id: 'sample-5000', timestampMs: 5_000 },
        limit: 1_000,
      });
    for (const timestampMs of [0, 5_000, 10_000]) {
      const marker = screen.getByRole('button', {
        name: new RegExp(`Visual frame ${timestampMs / 1_000}\\.0s`),
      });
      fireEvent.click(marker);
      await waitFor(() =>
        expect(ocrClient.getOcrDocument).toHaveBeenCalledWith({
          recordingId: detail.id,
          sourceTrackId: 'video-0',
          timestampMs,
          documentId: `sample-${timestampMs}`,
        }),
      );
      expect(screen.queryByLabelText('Recognized text')).toBeNull();
      fireEvent.click(
        await screen.findByRole('button', { name: 'Read OCR text' }),
      );
      expect(
        (await screen.findByLabelText('Recognized text')).textContent,
      ).toBe('Identical literal meeting text');
      await userEvent.keyboard('{Escape}');
      await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
      expect(
        screen
          .getByRole('slider', { name: 'Playback position' })
          .getAttribute('aria-valuenow'),
      ).toBe(String(timestampMs));
    }
  },
);

test('RecordingStudioScreen cuts an actual OCR sample and ignores an older observation index that resolves afterward', async () => {
  const sampledDetail = {
    ...longDetail,
    durationMs: 38_000,
    segments: [{ ...longDetail.segments[0]!, durationMs: 38_000 }],
  } satisfies LibraryRecordingDetail;
  const frames = [0, 5_000, 10_000].map((frameTimestampMs) => ({
    id: `sample-${frameTimestampMs}`,
    recordingId: detail.id,
    segmentId: 'segment-long',
    sourceTrackId: 'video-0',
    frameTimestampMs,
    sampleEndMs: frameTimestampMs + 1,
    sourceFrameRef: `opaque-sample-${frameTimestampMs}`,
    status: 'succeeded' as const,
    errorCode: null,
    documentId: `sample-${frameTimestampMs}`,
    ocrOutputCount: 1,
    textEmpty: false,
  }));
  client.getLibraryRecording.mockResolvedValue(sampledDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);
  ocrClient.getOcrTimelineFrames.mockResolvedValue({
    recordingId: detail.id,
    frames,
    nextCursor: null,
  });
  client.createLibraryRedactions.mockResolvedValue({
    ...sampledDetail,
    redactions: [
      {
        id: 'cut-five',
        createdAtMs: 1,
        recordingId: detail.id,
        startOffsetMs: 4_000,
        endOffsetMs: 6_000,
        reason: null,
        trackIds: ['video-0'],
      },
    ],
  });
  render(<RecordingStudioScreen />);
  await waitFor(() =>
    expect(
      screen.getAllByRole('button', { name: /Visual frame/i }),
    ).toHaveLength(3),
  );
  const stale = deferred<OcrTimelineFramePage>();
  ocrClient.getOcrTimelineFrames.mockReturnValue(stale.promise);
  fireEvent.click(screen.getByRole('button', { name: /^Refresh$/i }));
  await waitFor(() =>
    expect(screen.getByText('Loading OCR observation index…')).toBeTruthy(),
  );
  ocrClient.getOcrTimelineFrames.mockResolvedValue({
    recordingId: detail.id,
    frames: frames.filter((frame) => frame.frameTimestampMs !== 5_000),
    nextCursor: null,
  });
  const timeline = screen.getByLabelText('Track timeline');
  mockTimelineRect(timeline);
  const x = timelineClientXForMs(timeline, 5_000, 38_000);
  const ruler = screen.getByLabelText('Time ruler');
  fireEvent.pointerDown(ruler, {
    button: 0,
    clientX: x,
    clientY: 12,
    pointerId: 93,
  });
  fireEvent.pointerMove(timeline, {
    button: 0,
    clientX: timelineClientXForMs(timeline, 5_500, 38_000),
    clientY: 12,
    pointerId: 93,
  });
  fireEvent.pointerUp(timeline, {
    button: 0,
    clientX: timelineClientXForMs(timeline, 5_500, 38_000),
    clientY: 12,
    pointerId: 93,
  });
  fireEvent.click(screen.getAllByRole('button', { name: /^Redact$/i })[0]!);
  await waitFor(() =>
    expect(
      screen.getAllByRole('button', { name: /Visual frame/i }),
    ).toHaveLength(2),
  );
  stale.resolve({ recordingId: detail.id, frames, nextCursor: null });
  await waitFor(() =>
    expect(
      screen.queryByRole('button', { name: /Visual frame 5\.0s/i }),
    ).toBeNull(),
  );
  expect(
    screen
      .getByRole('button', { name: /Visual frame 0\.0s/i })
      .getAttribute('aria-label'),
  ).toContain('observation');
});
