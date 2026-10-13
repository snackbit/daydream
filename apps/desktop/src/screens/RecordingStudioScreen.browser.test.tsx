import type { ReactNode } from 'react';
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
  RecallEvidence,
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
import {
  controlContrast,
  settleVisualTransitions,
  textContrast,
} from '@/app/design-accessibility-audit';
import { useStudioTimelineStore } from '@/features/library/components/studio/studio-timeline-store';
import {
  TRACK_LABEL_WIDTH_PX,
  TRACK_LANE_HEIGHT_PX,
  TRACK_RULER_HEIGHT_PX,
} from '@/features/library/components/studio/track-timeline-math';
import syntheticMedia from '@/features/library/recording-studio-route/stories/fixtures/synthetic-player.webm?url';
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
  adjacentLibraryFrame: vi.fn(),
  resizeLibraryRedaction: vi.fn(),
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
  search: {
    evidenceId: null as string | null,
    source: null as 'ocr' | 'desktop-audio' | 'microphone' | null,
    seek: null as number | null,
  },
}));
const recallClient = vi.hoisted(() => ({
  resolveRecallEvidence: vi.fn(),
  listenToRecallStateChanged: vi.fn(),
}));
const debugState = vi.hoisted(() => ({ enabled: false }));
vi.mock('@/features/recall/recall-client', () => recallClient);

let originalViewport: { height: number; width: number } | null = null;
let layoutStylesheet: HTMLLinkElement | null = null;

vi.mock('@/features/library/library-client', () => client);
vi.mock('@/features/ai/ai-client', () => aiClient);
vi.mock('@/features/ai/ocr/ocr-client', () => ocrClient);
vi.mock('@/features/settings/use-debug-enabled', () => ({
  useDebugEnabled: () => debugState.enabled,
}));

vi.mock('@tanstack/react-router', () => ({
  Link: ({
    to,
    search,
    children,
  }: {
    to: string;
    search?: { tab: string; focus: string | null };
    children: ReactNode;
  }) => (
    <a
      href={search ? `${to}?tab=${search.tab}&focus=${search.focus ?? ''}` : to}
    >
      {children}
    </a>
  ),
  useNavigate: () => router.navigate,
  useParams: () => ({ recordingId: routeState.recordingId }),
  useSearch: () => routeState.search,
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

test('independent source selection and grid keep stream URLs and redactions isolated', async () => {
  const sourceA = '00000000-0000-4000-8000-000000000101',
    sourceB = '00000000-0000-4000-8000-000000000102';
  const streams = [
    {
      trackId: sourceA,
      sourceId: 'monitor-a',
      label: 'Screen A',
      streamIndex: 0,
      width: 1280,
      height: 720,
      fps: 2,
    },
    {
      trackId: sourceB,
      sourceId: 'monitor-b',
      label: 'Screen B',
      streamIndex: 1,
      width: 640,
      height: 720,
      fps: 2,
    },
  ];
  client.getLibraryRecording.mockResolvedValue({
    ...detail,
    tracks: [
      { ...detail.tracks[0]!, id: sourceA, label: 'Screen A', streamIndex: 0 },
      { ...detail.tracks[0]!, id: sourceB, label: 'Screen B', streamIndex: 1 },
      ...detail.tracks.filter((track) => track.kind !== 'video'),
    ],
    segments: detail.segments.map((segment) => ({
      ...segment,
      videoStreams: streams,
    })),
    redactions: [
      {
        id: 'source-a-cut',
        recordingId: detail.id,
        startOffsetMs: 0,
        endOffsetMs: 2000,
        reason: null,
        createdAtMs: 1,
        trackIds: [sourceA],
      },
    ],
  });
  client.createLibraryPlaybackSession.mockResolvedValue({
    ...playbackAssets,
    segments: playbackAssets.segments.map((segment) => ({
      ...segment,
      videos: [
        {
          trackId: sourceA,
          streamIndex: 0,
          url: `${new URL(syntheticMedia, window.location.origin).href}?fixture=source-a`,
          thumbnails: [],
        },
        {
          trackId: sourceB,
          streamIndex: 1,
          url: `${new URL(syntheticMedia, window.location.origin).href}?fixture=source-b`,
          thumbnails: [],
        },
      ],
    })),
  });
  const view = render(<RecordingStudioScreen />);
  fireEvent.click(
    await screen.findByRole('combobox', { name: 'Video source' }),
  );
  fireEvent.click(await screen.findByRole('option', { name: 'Screen B' }));
  await waitFor(() =>
    expect(
      view.container.querySelector('video')?.getAttribute('src'),
    ).toContain('fixture=source-b'),
  );
  await userEvent.click(screen.getByRole('combobox', { name: 'Video source' }));
  await userEvent.click(screen.getByRole('option', { name: 'All monitors' }));
  await screen.findByLabelText('Screen A synchronized video');
  expect(screen.getByLabelText('Screen A redacted')).toBeTruthy();
  expect(screen.queryByLabelText('Screen B redacted')).toBeNull();
  await userEvent.click(
    screen.getByRole('combobox', { name: 'Playback speed' }),
  );
  await userEvent.click(screen.getByRole('option', { name: '2x' }));
  await screen.findByRole('combobox', { name: 'Playback speed' });
  for (const video of [...view.container.querySelectorAll('video')].filter(
    (video) => !video.closest('[hidden]'),
  ))
    expect(video.playbackRate).toBe(2);
  expect(window.HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
  client.adjacentLibraryFrame.mockResolvedValue(500);
  fireEvent.keyDown(document.body, { key: '.', code: 'Period' });
  await waitFor(() =>
    expect(client.adjacentLibraryFrame).toHaveBeenCalledWith(
      detail.id,
      0,
      1,
      sourceB,
    ),
  );
  await waitFor(() =>
    expect(
      screen.getByRole('slider', { name: 'Playback position' }),
    ).toHaveAttribute('aria-valuenow', '500'),
  );
  vi.spyOn(
    window.HTMLMediaElement.prototype,
    'readyState',
    'get',
  ).mockReturnValue(HTMLMediaElement.HAVE_METADATA);
  await userEvent.click(screen.getByRole('button', { name: 'Play' }));
  await userEvent.click(screen.getByRole('combobox', { name: 'Video source' }));
  await userEvent.click(screen.getByRole('option', { name: 'Screen A' }));
  expect(await screen.findByRole('button', { name: 'Pause' })).toBeTruthy();
  expect(window.HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
  vi.spyOn(
    window.HTMLMediaElement.prototype,
    'readyState',
    'get',
  ).mockReturnValue(HTMLMediaElement.HAVE_ENOUGH_DATA);
  fireEvent.canPlay(view.container.querySelector('video')!);
  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalled(),
  );
});

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

function syntheticThumbnail(color: string) {
  const canvas = document.createElement('canvas');
  canvas.width = 320;
  canvas.height = 180;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('A synthetic thumbnail requires Canvas2D');
  context.fillStyle = color;
  context.fillRect(0, 0, canvas.width, canvas.height);
  return canvas.toDataURL('image/png');
}
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
          url: `${new URL(syntheticMedia, window.location.origin).href}?fixture=segment-1-audio`,
        },
      ],
      segmentId: 'segment-1',
      thumbnails: [
        {
          offsetMs: 0,
          url: syntheticThumbnail('#5287f9'),
        },
        {
          offsetMs: 1_000,
          url: syntheticThumbnail('#268958'),
        },
      ],
      videoUrl: `${new URL(syntheticMedia, window.location.origin).href}?fixture=segment-1-video`,
    },
    {
      audio: [
        {
          kind: 'desktop-audio',
          streamIndex: 1,
          trackId: 'desktop-audio-1',
          url: `${new URL(syntheticMedia, window.location.origin).href}?fixture=segment-2-audio`,
        },
      ],
      segmentId: 'segment-2',
      thumbnails: [
        {
          offsetMs: 0,
          url: syntheticThumbnail('#b87826'),
        },
      ],
      videoUrl: `${new URL(syntheticMedia, window.location.origin).href}?fixture=segment-2-video`,
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
          url: syntheticThumbnail('#5287f9'),
        },
      ],
      videoUrl: `${new URL(syntheticMedia, window.location.origin).href}?fixture=segment-long-video`,
    },
  ],
} satisfies LibraryPlaybackAssets;

test.each(['light', 'dark'] as const)(
  'Studio seekbar and redaction presentation stay readable in %s',
  async (theme) => {
    await loadStudioStyles();
    document.documentElement.dataset.theme = theme;
    client.getLibraryRecording.mockResolvedValue({
      ...longDetail,
      redactions: [
        {
          id: 'visual-mask',
          recordingId: detail.id,
          startOffsetMs: 50000,
          endOffsetMs: 100000,
          createdAtMs: 1,
          reason: null,
          trackIds: ['video-0', 'desktop-audio-1'],
        },
      ],
    });
    client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);
    render(<RecordingStudioScreen />);
    const seekbar = await screen.findByRole('slider', {
      name: 'Playback position',
    });
    await settleVisualTransitions();
    const region = seekbar.querySelector('[data-playback-region="segment"]');
    if (!region) throw new Error('Missing visible seekbar region');
    // Intentional visual regression: the reported white-on-beige seek track must
    // have non-text control contrast in both appearances.
    expect(controlContrast(region, 'backgroundColor')).toBeGreaterThanOrEqual(
      3,
    );
    const video = screen
      .getByRole('region', { name: 'Recording player' })
      .querySelector('video');
    if (!video) throw new Error('Missing player video');
    fireEvent.error(video);
    const failure = await screen.findByText('Playback failed', { exact: true });
    expect(textContrast(failure)).toBeGreaterThanOrEqual(4.5);
    const mask = screen.getByRole('button', { name: /Video redacted range/i });
    await userEvent.click(mask);
    expect(mask.getAttribute('aria-pressed')).toBe('true');
    await page.screenshot({
      path: `__screenshots__/studio-redaction-${theme}.png`,
    });
    seekbar.focus();
    await userEvent.keyboard('{ArrowRight}');
    await waitFor(() =>
      expect(
        screen
          .getByRole('slider', { name: 'Playback position' })
          .getAttribute('aria-valuenow'),
      ).toBe('5000'),
    );
    expect(
      screen.getByRole('region', { name: 'Recording player' }),
    ).toBeTruthy();
  },
);

beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  debugState.enabled = false;
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
  // IPC media URLs are synthetic; model the decoded media boundary explicitly.
  vi.spyOn(
    window.HTMLMediaElement.prototype,
    'readyState',
    'get',
  ).mockReturnValue(HTMLMediaElement.HAVE_ENOUGH_DATA);
  vi.spyOn(window.HTMLMediaElement.prototype, 'seeking', 'get').mockReturnValue(
    false,
  );
  routeState.recordingId = 'recording-1';
  routeState.search = { evidenceId: null, source: null, seek: null };
  recallClient.resolveRecallEvidence.mockResolvedValue(null);
  recallClient.listenToRecallStateChanged.mockResolvedValue(() => undefined);
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
  delete document.documentElement.dataset.theme;
});

async function setTestViewport(width: number, height: number) {
  originalViewport ??= { height: window.innerHeight, width: window.innerWidth };
  await page.viewport(width, height);
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

async function inspectScreenAt(
  timestampMs: number,
  durationMs = detail.durationMs,
) {
  const slider = await screen.findByRole('slider', {
    name: 'Playback position',
  });
  mockScrubberRect(slider);
  vi.spyOn(slider, 'setPointerCapture').mockImplementation(() => undefined);
  const clientX = (timestampMs / durationMs) * 700;
  fireEvent.pointerDown(slider, { button: 0, clientX, pointerId: 71 });
  fireEvent.pointerUp(slider, { button: 0, clientX, pointerId: 71 });
  await userEvent.click(screen.getByRole('button', { name: 'Select Video' }));
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

async function mockTimelineRect(timeline: HTMLElement) {
  await new Promise<void>((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
  );
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
  const timelineWidthPx =
    Number.parseFloat(timeline.style.width) - TRACK_LABEL_WIDTH_PX;
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
  debugState.enabled = true;
  render(<RecordingStudioScreen />);

  expect(await screen.findByText('Tracks')).toBeTruthy();
  expect(screen.getByRole('button', { name: /^play$/i })).toBeTruthy();
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
  ).toHaveLength(0);
  expect(screen.getByText('Desktop audio')).toBeTruthy();
  await userEvent.click(screen.getByRole('tab', { name: 'Details' }));
  expect(
    screen.getByRole('heading', { name: 'Recording details' }),
  ).toBeTruthy();
  expect(screen.getByText('recording-1')).toBeTruthy();
  expect(await screen.findByLabelText(/mute desktop audio/i)).toBeTruthy();
  expect(
    await screen.findByRole('img', { name: 'Desktop audio waveform' }),
  ).toBeTruthy();
  expect(screen.queryByRole('button', { name: /Video segment/i })).toBeNull();
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

test('assembled Studio forwards overview counts and padded zoom through sortable speech and activity tracks', async () => {
  const durationMs = 28_800_000;
  client.getLibraryRecording.mockResolvedValue({
    ...longDetail,
    durationMs,
    segments: [{ ...longDetail.segments[0]!, durationMs }],
  });
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);
  const overviewState: RecordingAiProcessingState = {
    recordingId: detail.id,
    job: null,
    chunks: [],
    transcript: { recordingId: detail.id, tracks: [] },
    timelineOverview: [
      {
        trackId: 'desktop-audio-1',
        kind: 'speech',
        startOffsetMs: 0,
        endOffsetMs: 1000,
        count: 20,
      },
      {
        trackId: 'desktop-audio-1',
        kind: 'speech',
        startOffsetMs: 2000,
        endOffsetMs: 3000,
        count: 30,
      },
      {
        trackId: 'activity:mouse',
        kind: 'activity',
        startOffsetMs: 0,
        endOffsetMs: 3000,
        count: 12,
      },
    ],
  };
  const excerpt = {
    id: 'zoom-excerpt',
    segmentId: longDetail.segments[0]!.id,
    startOffsetMs: 0,
    endOffsetMs: 3000,
    text: 'Synthetic speech exposed by zoom',
    words: [],
    confidence: null,
    provider: 'local-whisper-cpp',
    model: null,
  };
  const detailState: RecordingAiProcessingState = {
    recordingId: detail.id,
    chunks: [],
    job: null,
    transcript: {
      recordingId: detail.id,
      tracks: [
        {
          trackId: 'desktop-audio-1',
          sourceRole: 'desktop-audio',
          segments: [excerpt],
        },
      ],
    },
    transcriptionTracks: [
      {
        trackId: 'desktop-audio-1',
        libraryTrackId: 'desktop-audio-1',
        trackKey: 'desktop-audio',
        sourceRole: 'desktop-audio',
        label: 'Desktop audio',
        description: '',
        textSegmentCount: 1,
        status: 'complete',
        statusLabel: 'Complete',
        chunks: [
          {
            id: 'zoom-chunk',
            segmentId: excerpt.segmentId,
            startOffsetMs: 0,
            endOffsetMs: 3000,
            attemptCount: 1,
            progress: 1,
            result: 'text',
            status: 'succeeded',
            text: excerpt.text,
            transcriptSegments: [excerpt],
            errorCode: null,
            resultCode: null,
          },
        ],
      },
    ],
  };
  aiClient.getRecordingAiProcessingState.mockImplementation(
    async (input: { bucketMs?: number }) =>
      input.bucketMs === undefined ? detailState : overviewState,
  );
  render(<RecordingStudioScreen />);
  const speech = await screen.findByRole('button', {
    name: 'Zoom into 50 transcript entries',
  });
  const activity = await screen.findByRole('button', {
    name: 'Zoom into 12 activity events',
  });
  const initialZoom = screen.getByLabelText('Track timeline').style.width;
  await userEvent.click(activity);
  await waitFor(() =>
    expect(screen.getByLabelText('Track timeline').style.width).not.toBe(
      initialZoom,
    ),
  );
  const toolbar = document.querySelector('[data-studio-timeline-toolbar]');
  if (!(toolbar instanceof HTMLElement))
    throw new Error('Studio timeline toolbar is missing');
  expect(
    within(toolbar).getByRole('button', { name: 'Follow playback' }),
  ).toBeTruthy();
  expect(speech).toBeTruthy();
  expect(screen.getByRole('tab', { name: 'Review' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  const transcriptButton = await screen.findByRole('button', {
    name: /Inspect Desktop audio transcript/,
  });
  expect(transcriptButton).toHaveTextContent(excerpt.text);
  await userEvent.click(transcriptButton);
  expect(
    await screen.findByRole('region', { name: 'Desktop audio transcript' }),
  ).toHaveTextContent(excerpt.text);
  expect(screen.getByRole('tab', { name: 'Details' })).toHaveAttribute(
    'aria-selected',
    'true',
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
    await screen.findByRole('img', { name: /Microphone waveform/ }),
  ).toBeTruthy();
  expect(
    screen.queryByRole('button', { name: /Segment 3|Video segment/i }),
  ).toBeNull();
});

test('RecordingStudioScreen starts AI processing from the detail header', async () => {
  render(<RecordingStudioScreen />);

  expect(await screen.findByText('Tracks')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: /process recording/i }));

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
  debugState.enabled = true;
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

  await page.getByRole('tab', { name: 'Details' }).click();
  await waitFor(() =>
    expect(screen.getAllByText('recording-2').length).toBeGreaterThan(0),
  );

  const processButton = screen.getByRole('button', {
    name: /process recording/i,
  });
  expect((processButton as HTMLButtonElement).disabled).toBe(false);
  expect(screen.queryByRole('button', { name: /processed/i })).toBeNull();
});

test('opening another recording resets speed to 1x and replaces the media group', async () => {
  client.getLibraryRecording.mockImplementation((recordingId: string) =>
    Promise.resolve({ ...detail, id: recordingId }),
  );
  client.createLibraryPlaybackSession.mockImplementation(
    ({ recordingId }: { recordingId: string }) =>
      Promise.resolve({ ...playbackAssets, recordingId }),
  );
  const view = render(<RecordingStudioScreen />);
  await userEvent.click(
    await screen.findByRole('combobox', { name: 'Playback speed' }),
  );
  await userEvent.click(screen.getByRole('option', { name: '2x' }));
  const original = view.container.querySelector('video');
  expect(original?.playbackRate).toBe(2);
  routeState.recordingId = 'recording-2';
  view.rerender(<RecordingStudioScreen />);
  await waitFor(() => {
    const replacement = view.container.querySelector('video');
    expect(replacement).toBeTruthy();
    expect(replacement).not.toBe(original);
    expect(replacement?.playbackRate).toBe(1);
  });
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
    name: /^processing\.\.\.$/i,
  });

  expect((processingButton as HTMLButtonElement).disabled).toBe(true);
  expect(screen.queryByRole('button', { name: /processed/i })).toBeNull();
  expect(
    screen.getByRole('progressbar', {
      name: /Desktop audio · Transcription · Processing/,
    }),
  ).toBeTruthy();
});

test('RecordingStudioScreen updates AI lanes from processing events', async () => {
  const processingListeners = new Set<
    (state: RecordingAiProcessingState) => void
  >();
  const processingHandler = (state: RecordingAiProcessingState) =>
    processingListeners.forEach((handler) => handler(state));
  aiClient.listenToAiProcessingChanged.mockImplementation((handler) => {
    processingListeners.add(handler);
    return Promise.resolve(() => {
      processingListeners.delete(handler);
    });
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
    await screen.findByRole('progressbar', {
      name: /Desktop audio · Transcription · Processing/,
    }),
  ).toBeTruthy();
  expect(screen.queryByText('Transcription in progress')).toBeNull();
  expect(
    (
      screen.getByRole('button', {
        name: /^processing\.\.\.$/i,
      }) as HTMLButtonElement
    ).disabled,
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

  expect((await screen.findAllByText('Standup notes')).length).toBeGreaterThan(
    0,
  );
  expect(screen.queryByText('Transcription failed')).toBeNull();
  expect(
    screen.getByRole('progressbar', {
      name: /Desktop audio · Transcription · Needs attention/,
    }),
  ).toBeTruthy();
  expect(
    (
      screen.getByRole('button', {
        name: /retry failed sections/i,
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(false);
});

test('RecordingStudioScreen explains a legacy transcription failure and links to its configuration', async () => {
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
    (await screen.findAllByText(/provider could not process this section/i))[0],
  ).toBeTruthy();
  expect(
    screen.getAllByRole('link', { name: 'Review transcription settings' })[0],
  ).toBeTruthy();
  expect(screen.queryByText('ai_transcription_provider_failed')).toBeNull();
  expect(
    screen
      .getAllByRole('link', { name: 'Review transcription settings' })[0]!
      .getAttribute('href'),
  ).toBe('/settings?tab=ai&focus=transcription');
  await page.getByRole('button', { name: /retry failed sections/i }).click();
  await waitFor(() =>
    expect(aiClient.retryProcessingJob).toHaveBeenCalledWith({
      jobId: 'job-1',
    }),
  );
  expect(aiClient.startRecordingAiProcessing).not.toHaveBeenCalled();
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
  expect(markers).toHaveLength(2);

  const marker = screen.getByRole('button', {
    name: /visual frame 0\.5s to 1\.0s, 1 vlm, 2 ocr candidates/i,
  });
  const ocrMarker = screen.queryByRole('button', {
    name: /visual frame 1\.0s to 1\.5s, no vlm, 1 ocr candidate/i,
  });
  const vlmMarker = screen.getByRole('button', {
    name: /visual frame 1\.5s to 2\.0s, 1 vlm, 0 ocr candidates/i,
  });
  expect(marker.textContent).toContain('0.5s');
  expect(marker.textContent).not.toMatch(/OCR 2|VLM 1/i);
  expect(marker.getAttribute('data-visual-frame-source')).toBe('mixed');
  expect(ocrMarker).toBeNull();
  expect(vlmMarker.textContent).toContain('1.5s');
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
    (
      await screen.findAllByText(
        'A browser checkout page is visible with a payment form and order summary.',
      )
    ).length >= 2,
  ).toBeTruthy();
  expect(screen.queryByLabelText('Recognized text')).toBeNull();
  await screen.findByText(/Screen recognition complete/);
  expect(screen.queryByText('Checkout Total')).toBeNull();
  expect(screen.queryByRole('button', { name: 'Read OCR text' })).toBeNull();
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
  expect(
    (await screen.findAllByText('private spoken secret')).length,
  ).toBeGreaterThan(0);
  await inspectScreenAt(1700);
  await screen.findByText(/Screen recognition complete/);
  expect(screen.queryByText('private screen secret')).toBeNull();
  expect(screen.queryByRole('button', { name: 'Read OCR text' })).toBeNull();
  expect(screen.queryByText('Frame reference')).toBeNull();

  const timeline = await screen.findByLabelText('Track timeline');
  await mockTimelineRect(timeline);
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
  debugState.enabled = true;
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

  expect((await screen.findAllByText('First words')).length).toBeGreaterThan(0);
  expect(screen.getAllByText('Later words').length).toBeGreaterThan(0);
  expect(
    screen.getAllByRole('button', {
      name: /Inspect Desktop audio transcript/i,
    }),
  ).toHaveLength(2);

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
  expect(viewport.scrollTop).toBe(0);

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

test('RecordingStudioScreen batches visible waveform data and reuses it after remount', async () => {
  routeState.recordingId = 'waveform-batch-recording';
  const recording = { ...longDetail, id: routeState.recordingId };
  client.getLibraryRecording.mockResolvedValue(recording);
  client.createLibraryPlaybackSession.mockResolvedValue({
    ...longPlaybackAssets,
    recordingId: recording.id,
  });
  client.getRecordingWaveformTiles.mockResolvedValue({
    ...waveformTiles,
    recordingId: recording.id,
  });
  const view = render(<RecordingStudioScreen />);
  await screen.findByRole('img', { name: 'Desktop audio waveform' });
  await mockTimelineRect(screen.getByLabelText('Track timeline'));
  expect(client.getRecordingWaveformTiles).toHaveBeenCalled();
  expect(
    client.getRecordingWaveformTiles.mock.calls[0]![0].endTimelineMs,
  ).toBeGreaterThan(15_000);
  const reads = client.getRecordingWaveformTiles.mock.calls.length;
  const readers = [
    aiClient.getRecordingAiProcessingState,
    aiClient.getRecordingActivity,
    aiClient.getRecordingVisualObservations,
    ocrClient.getOcrTimelineFrames,
  ];
  const counts = readers.map((reader) => reader.mock.calls.length);
  await act(async () => {
    for (const [listener] of aiClient.listenToAiProcessingChanged.mock.calls)
      listener({
        recordingId: recording.id,
        sourceKind: 'waveform',
        updatedAtMs: 1,
      });
  });
  await new Promise((resolve) => window.setTimeout(resolve, 550));
  expect(readers.map((reader) => reader.mock.calls.length)).toEqual(counts);
  expect(client.getRecordingWaveformTiles.mock.calls.length).toBe(reads);
  view.unmount();
  render(<RecordingStudioScreen />);
  await screen.findByRole('img', { name: 'Desktop audio waveform' });
  expect(client.getRecordingWaveformTiles.mock.calls.length).toBe(reads);
});

test('RecordingStudioScreen commits ruler seeks through the playback pipeline', async () => {
  client.getLibraryRecording.mockResolvedValue(longDetail);
  client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);

  const { container } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${longPlaybackAssets.segments[0]!.videoUrl}&startMs=0&request=0`,
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
      `${longPlaybackAssets.segments[0]!.videoUrl}&startMs=`,
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

  await userEvent.click(
    screen.getByRole('button', { name: 'Select Desktop audio' }),
  );

  const trackName = await screen.findByLabelText('Track name');
  expect((trackName as HTMLInputElement).value).toBe('Desktop audio');
});

test('RecordingStudioScreen ctrl-click adds a source from its header', async () => {
  render(<RecordingStudioScreen />);
  fireEvent.click(await screen.findByRole('button', { name: 'Select Video' }));
  fireEvent.click(
    screen.getByRole('button', { name: 'Select Desktop audio' }),
    { ctrlKey: true },
  );
  expect(
    (await screen.findAllByLabelText('Track name')).map(
      (input) => (input as HTMLInputElement).value,
    ),
  ).toEqual(['Video', 'Desktop audio']);
});

test('RecordingStudioScreen clears selection on a plain timeline click', async () => {
  render(<RecordingStudioScreen />);
  expect(await screen.findByRole('tab', { name: 'Review' })).toBeTruthy();

  fireEvent.click(
    await screen.findByRole('button', {
      name: /^Select Desktop audio$/i,
    }),
  );
  expect(await screen.findByLabelText('Track name')).toBeTruthy();
  expect(screen.getByRole('tab', { name: 'Details' })).toHaveAttribute(
    'aria-selected',
    'true',
  );

  const timeline = await screen.findByLabelText('Track timeline');
  await mockTimelineRect(timeline);
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
  expect(screen.getByRole('tab', { name: 'Details' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
});

test('RecordingStudioScreen selects all lanes from a ruler drag', async () => {
  render(<RecordingStudioScreen />);

  const timeline = await screen.findByLabelText('Track timeline');
  await mockTimelineRect(timeline);
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
  await mockTimelineRect(timeline);
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
  expect(await screen.findAllByText('Redacted')).toHaveLength(1);
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
  await mockTimelineRect(timeline);
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
  await mockTimelineRect(timeline);
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
  await mockTimelineRect(timeline);
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
  // Pointer coordinates must use the settled ResizeObserver viewport width.
  await new Promise<void>((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
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
  await mockTimelineRect(timeline);
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
  await mockTimelineRect(timeline);
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
  await mockTimelineRect(timeline);
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

test('RecordingStudioScreen presents one continuous audio source without storage clip editing', async () => {
  render(<RecordingStudioScreen />);
  await userEvent.click(
    await screen.findByRole('button', { name: 'Select Desktop audio' }),
  );
  expect(
    await screen.findByRole('textbox', { name: 'Track name' }),
  ).toHaveValue('Desktop audio');
  expect(screen.queryByLabelText('Clip title')).toBeNull();
  expect(
    screen.queryByRole('button', { name: /Desktop audio Segment/ }),
  ).toBeNull();
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
        videoUrl: `${new URL(syntheticMedia, window.location.origin).href}?fixture=segment-3-video`,
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
  expect(screen.queryByText(/Not recorded. Next recording at/i)).toBeNull();
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
  audio.currentTime = 0.95;
  await userEvent.click(screen.getByRole('button', { name: /^play$/i }));
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
      `${playbackAssets.segments[0]!.videoUrl}&startMs=1500&request=1`,
    ),
  );

  const skippedVideo = container.querySelector('video')!;
  skippedVideo.currentTime = 1.5;
  fireEvent.timeUpdate(skippedVideo);

  await waitFor(() => {
    expect(
      within(skippedVideo.parentElement!).queryByText('Redacted'),
    ).toBeNull();
    expect(container.querySelector('audio')?.muted).toBe(false);
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
  await userEvent.click(screen.getByRole('button', { name: /^play$/i }));
  Object.defineProperty(video, 'ended', { configurable: true, value: true });
  fireEvent.play(video);
  fireEvent.pause(video);
  vi.mocked(window.HTMLMediaElement.prototype.play).mockClear();
  fireEvent.ended(video);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      playbackAssets.segments[1]!.videoUrl,
    ),
  );

  fireEvent.canPlay(container.querySelector('video')!);

  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalled(),
  );
});

test('RecordingStudioScreen commits a scrubbed seek and waits for canplay before resuming', async () => {
  const { container } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}&startMs=0&request=0`,
    ),
  );

  await userEvent.click(screen.getByRole('button', { name: /^play$/i }));
  vi.mocked(window.HTMLMediaElement.prototype.play).mockClear();
  vi.spyOn(
    window.HTMLMediaElement.prototype,
    'readyState',
    'get',
  ).mockReturnValue(HTMLMediaElement.HAVE_METADATA);
  const slider = screen.getByRole('slider', { name: /playback position/i });
  mockScrubberRect(slider);
  fireEvent.pointerDown(slider, { clientX: 0, pointerId: 1 });
  fireEvent.pointerMove(slider, { clientX: 100, pointerId: 1 });

  expect(container.querySelector('video')?.getAttribute('src')).toContain(
    `${playbackAssets.segments[0]!.videoUrl}&startMs=0&request=0`,
  );

  fireEvent.pointerUp(slider, { clientX: 100, pointerId: 1 });

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}&startMs=1000&request=1`,
    ),
  );
  expect(container.querySelector('audio')?.getAttribute('src')).toContain(
    `${playbackAssets.segments[0]!.audio[0]!.url}&startMs=1000&request=1`,
  );
  expect(window.HTMLMediaElement.prototype.play).not.toHaveBeenCalled();

  vi.spyOn(
    window.HTMLMediaElement.prototype,
    'readyState',
    'get',
  ).mockReturnValue(HTMLMediaElement.HAVE_ENOUGH_DATA);
  fireEvent.canPlay(container.querySelector('video')!);

  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalled(),
  );
});

test('RecordingStudioScreen seeks while paused without autoplaying', async () => {
  const { container } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}&startMs=0&request=0`,
    ),
  );

  const slider = screen.getByRole('slider', { name: /playback position/i });
  mockScrubberRect(slider);
  fireEvent.pointerDown(slider, { clientX: 100, pointerId: 1 });
  fireEvent.pointerUp(slider, { clientX: 100, pointerId: 1 });

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}&startMs=1000&request=1`,
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

  expect(
    within(await screen.findByRole('tooltip')).getAllByText('0:01').length,
  ).toBe(1);
  expect(
    screen.getByRole('tooltip').querySelector('img')?.getAttribute('src'),
  ).toBe(playbackAssets.segments[0]!.thumbnails[1]!.url);

  fireEvent.pointerMove(slider, { clientX: 300, pointerId: 1 });

  expect(await screen.findAllByText('Paused')).toHaveLength(1);
  expect(screen.getByText(/0:03 gap/i)).toBeTruthy();
});

test('RecordingStudioScreen jumps paused-gap seeks to the next segment', async () => {
  const { container } = render(<RecordingStudioScreen />);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}&startMs=0&request=0`,
    ),
  );

  const slider = screen.getByRole('slider', { name: /playback position/i });
  mockScrubberRect(slider);
  fireEvent.pointerDown(slider, { clientX: 300, pointerId: 1 });
  fireEvent.pointerUp(slider, { clientX: 300, pointerId: 1 });

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[1]!.videoUrl}&startMs=0&request=1`,
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
  expect(screen.getByText('Playback could not start. Try again.')).toBeTruthy();
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
      `${playbackAssets.segments[0]!.videoUrl}&startMs=0&request=0`,
    ),
  );

  vi.spyOn(
    window.HTMLMediaElement.prototype,
    'readyState',
    'get',
  ).mockReturnValue(HTMLMediaElement.HAVE_METADATA);
  fireEvent.ended(container.querySelector('video')!);
  fireEvent.click(screen.getByRole('button', { name: /^play$/i }));

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${playbackAssets.segments[0]!.videoUrl}&startMs=0&request=1`,
    ),
  );
  expect(window.HTMLMediaElement.prototype.play).not.toHaveBeenCalled();

  vi.spyOn(
    window.HTMLMediaElement.prototype,
    'readyState',
    'get',
  ).mockReturnValue(HTMLMediaElement.HAVE_ENOUGH_DATA);
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

  await userEvent.click(screen.getByRole('button', { name: /^play$/i }));
  vi.mocked(window.HTMLMediaElement.prototype.play).mockClear();
  fireEvent.ended(container.querySelector('video')!);

  expect(screen.getByText(/Not recorded. Next recording at/i)).toBeTruthy();
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
  fireEvent.canPlay(container.querySelector('video')!);
  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalled(),
  );
  await waitFor(() => {
    expect(screen.queryByText(/Not recorded. Next recording at/i)).toBeNull();
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

  await userEvent.click(screen.getByRole('button', { name: /^play$/i }));
  vi.mocked(window.HTMLMediaElement.prototype.play).mockClear();
  const retired = container.querySelector('video')!;
  fireEvent.ended(retired);
  // The old elements remain visible during a recorded pause gap, but their
  // late browser events must not start, fail or advance the retired stream.
  fireEvent.play(retired);
  fireEvent.pause(retired);
  fireEvent.timeUpdate(retired);
  fireEvent.error(retired);
  fireEvent.ended(retired);
  expect(screen.queryByRole('button', { name: 'Retry playback' })).toBeNull();
  expect(container.querySelector('video')).toBe(retired);
  fireEvent.click(screen.getByRole('button', { name: /play now/i }));

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      playbackAssets.segments[1]!.videoUrl,
    ),
  );
  fireEvent.canPlay(container.querySelector('video')!);
  await waitFor(() =>
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalled(),
  );
  expect(screen.queryByText(/Not recorded. Next recording at/i)).toBeNull();
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

async function loadStudioStyles() {
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
}

test('RecordingStudioScreen keeps the desktop track workspace tall enough to edit', async () => {
  await loadStudioStyles();
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

test('RecordingStudioScreen retains playback time and track edits while opening and closing its inspector', async () => {
  await setTestViewport(620, 960);
  const { container } = render(
    <div className="h-dvh w-full">
      <RecordingStudioScreen />
    </div>,
  );

  await screen.findByRole('button', { name: 'Review & details' });
  const video = container.querySelector('video');
  if (!video) {
    throw new Error('Expected the recording player to mount.');
  }

  Object.defineProperty(video, 'currentTime', {
    configurable: true,
    value: 1.25,
  });
  fireEvent.timeUpdate(video);
  act(() => useStudioTimelineStore.getState().selectLane('video-0'));
  await userEvent.click(
    screen.getByRole('button', { name: 'Review & details' }),
  );
  const inspector = await screen.findByRole('dialog', {
    name: 'Recording review',
  });
  const trackName = within(inspector).getByRole('textbox', {
    name: 'Track name',
  });
  await userEvent.fill(trackName, 'Edited video');

  expect(container.querySelector('video')).toBe(video);
  expect(video.currentTime).toBe(1.25);
  await userEvent.keyboard('{Escape}');
  expect(screen.queryByRole('dialog', { name: 'Recording review' })).toBeNull();
  await waitFor(() =>
    expect(document.activeElement).toBe(
      screen.getByRole('button', { name: 'Review & details' }),
    ),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Review & details' }),
  );
  expect(
    screen.getByRole<HTMLInputElement>('textbox', { name: 'Track name' }).value,
  ).toBe('Edited video');

  expect(container.querySelector('video')).toBe(video);
  expect(video.currentTime).toBe(1.25);
  expect(
    await screen.findByRole('group', { name: 'Track timeline' }),
  ).toBeTruthy();
});

test.each([
  ['light', 1440],
  ['dark', 1440],
  ['light', 1024],
  ['dark', 1024],
  ['light', 768],
  ['dark', 768],
  ['light', 390],
  ['dark', 390],
  ['light', 320],
  ['dark', 320],
] as const)(
  'Studio playback, zoom and inspector editing remain reachable in %s at %i pixels',
  async (theme, width) => {
    client.getLibraryRecording.mockResolvedValue(longDetail);
    client.createLibraryPlaybackSession.mockResolvedValue(longPlaybackAssets);
    await loadStudioStyles();
    await setTestViewport(width, 960);
    document.documentElement.dataset.theme = theme;
    const view = render(<RecordingStudioScreen />);
    const play = await screen.findByRole('button', { name: 'Play' });
    expect(
      screen.getByRole('slider', { name: 'Playback position' }),
    ).toBeTruthy();
    await userEvent.click(
      screen.getByRole('button', { name: 'Zoom in tracks' }),
    );
    expect(
      screen
        .getByRole('button', { name: 'Zoom out tracks' })
        .hasAttribute('disabled'),
    ).toBe(false);
    act(() => useStudioTimelineStore.getState().selectLane('video-0'));
    if (width < 1200)
      await userEvent.click(
        screen.getByRole('button', { name: 'Review & details' }),
      );
    const name = await screen.findByRole('textbox', { name: 'Track name' });
    await userEvent.fill(name, 'Synthetic edited screen');
    await page.screenshot({
      element:
        width < 1200
          ? screen.getByRole('dialog', { name: 'Recording review' })
          : view.container,
    });
    if (width < 1200)
      await userEvent.click(
        within(
          screen.getByRole('dialog', { name: 'Recording review' }),
        ).getByRole('button', { name: 'Close' }),
      );
    await userEvent.click(play);
    expect(await screen.findByRole('button', { name: 'Pause' })).toBeTruthy();
    await page.screenshot({ element: view.container });
  },
);

test('recall links resolve the exact source and seek once without autoplay, then withhold invalidated evidence', async () => {
  debugState.enabled = true;
  const evidence: RecallEvidence = {
    evidenceId: 'synthetic-evidence',
    recordingId: detail.id,
    segmentId: 'segment-1',
    sourceKind: 'ocr',
    sourceTrackId: 'screen-b',
    sourceId: 'ocr-source',
    sourceFrameRef: 'synthetic-frame',
    documentId: 'document',
    startOffsetMs: 1000,
    endOffsetMs: 1500,
    absoluteStartMs: 2000,
    absoluteEndMs: 2500,
    contentRevision: 'revision-1',
    text: 'Synthetic recall source text',
  };
  routeState.search = {
    evidenceId: evidence.evidenceId,
    source: 'ocr',
    seek: 1000,
  };
  const secondary = {
    ...detail.tracks[0]!,
    id: 'screen-b',
    label: 'Screen B',
    streamIndex: 3,
  };
  client.getLibraryRecording.mockResolvedValue({
    ...detail,
    tracks: [...detail.tracks, secondary],
    segments: detail.segments.map((segment) => ({
      ...segment,
      videoStreams: [
        {
          trackId: 'video-0',
          sourceId: 'screen-a',
          label: 'Screen A',
          streamIndex: 0,
          width: 1280,
          height: 720,
          fps: 2,
        },
        {
          trackId: 'screen-b',
          sourceId: 'screen-b',
          label: 'Screen B',
          streamIndex: 3,
          width: 1280,
          height: 720,
          fps: 2,
        },
      ],
    })),
  });
  client.createLibraryPlaybackSession.mockResolvedValue({
    ...playbackAssets,
    segments: playbackAssets.segments.map((segment) => ({
      ...segment,
      videos: [
        {
          trackId: 'video-0',
          label: 'Screen A',
          url: segment.videoUrl,
          thumbnails: segment.thumbnails,
        },
        {
          trackId: 'screen-b',
          label: 'Screen B',
          url: `${segment.videoUrl}?source=screen-b`,
          thumbnails: [],
        },
      ],
    })),
  });
  recallClient.resolveRecallEvidence.mockResolvedValue(evidence);
  let invalidate: (() => void) | undefined;
  recallClient.listenToRecallStateChanged.mockImplementation(
    async (handler: () => void) => {
      invalidate = handler;
      return () => undefined;
    },
  );
  const view = render(<RecordingStudioScreen />);
  await screen.findByText('Recall source');
  expect(screen.queryByText(evidence.text)).toBeNull();
  expect(
    screen.getByRole('combobox', { name: 'Video source' }).textContent,
  ).toContain('Screen B');
  expect(
    within(screen.getByRole('region', { name: 'Recall source' })).getByText(
      'screen-b',
    ),
  ).toBeTruthy();
  await waitFor(() =>
    expect(ocrClient.getOcrDocument).toHaveBeenCalledWith({
      recordingId: detail.id,
      sourceTrackId: 'screen-b',
      timestampMs: 1000,
      documentId: 'document',
    }),
  );
  expect(screen.getByRole('button', { name: 'Play' })).toBeTruthy();
  expect(window.HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Forward 10 seconds' }));
  const position = screen
    .getByRole('slider', { name: 'Playback position' })
    .getAttribute('aria-valuenow');
  view.rerender(<RecordingStudioScreen />);
  expect(
    screen
      .getByRole('slider', { name: 'Playback position' })
      .getAttribute('aria-valuenow'),
  ).toBe(position);
  recallClient.resolveRecallEvidence.mockResolvedValue(null);
  act(() => invalidate?.());
  await screen.findByText(/This recall source is unavailable/);
  expect(screen.queryByText(evidence.text)).toBeNull();
});

test.each([
  'missing',
  'wrong-recording',
  'wrong-source',
  'wrong-seek',
] as const)(
  'recall link %s remains unavailable without exposing or seeking evidence',
  async (scenario) => {
    routeState.search = {
      evidenceId: 'synthetic-evidence',
      source: 'ocr',
      seek: 1000,
    };
    const evidence: RecallEvidence = {
      evidenceId: 'synthetic-evidence',
      recordingId: scenario === 'wrong-recording' ? 'other' : detail.id,
      segmentId: 'segment-1',
      sourceKind: scenario === 'wrong-source' ? 'desktop-audio' : 'ocr',
      sourceTrackId: 'video-0',
      sourceId: 'ocr-source',
      sourceFrameRef: 'synthetic-frame',
      documentId: 'document',
      startOffsetMs: scenario === 'wrong-seek' ? 500 : 1000,
      endOffsetMs: 1500,
      absoluteStartMs: 2000,
      absoluteEndMs: 2500,
      contentRevision: 'revision-1',
      text: 'Withheld synthetic evidence',
    };
    recallClient.resolveRecallEvidence.mockResolvedValue(
      scenario === 'missing' ? null : evidence,
    );
    render(<RecordingStudioScreen />);
    await screen.findByText(/This recall source is unavailable/);
    expect(screen.queryByText(evidence.text)).toBeNull();
    expect(screen.getByRole('button', { name: 'Play' })).toBeTruthy();
    expect(window.HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
  },
);

test('hiding a recall source reports a failed write and retries its exact interval', async () => {
  const evidence: RecallEvidence = {
    evidenceId: 'synthetic-evidence',
    recordingId: detail.id,
    segmentId: 'segment-1',
    sourceKind: 'ocr',
    sourceTrackId: 'video-0',
    sourceId: 'ocr-source',
    sourceFrameRef: 'synthetic-frame',
    documentId: 'document',
    startOffsetMs: 1000,
    endOffsetMs: 1500,
    absoluteStartMs: 2000,
    absoluteEndMs: 2500,
    contentRevision: 'revision',
    text: 'Synthetic hideable source',
  };
  routeState.search = {
    evidenceId: evidence.evidenceId,
    source: 'ocr',
    seek: 1000,
  };
  recallClient.resolveRecallEvidence.mockResolvedValue(evidence);
  client.createLibraryRedactions.mockRejectedValueOnce(
    new Error('Synthetic write failure'),
  );
  render(<RecordingStudioScreen />);
  await screen.findByText('Recall source');
  expect(screen.queryByText(evidence.text)).toBeNull();
  await userEvent.click(
    screen.getByRole('button', { name: 'Hide source interval' }),
  );
  await screen.findByText('This source could not be hidden. Try again.');
  expect(screen.queryByText(evidence.text)).toBeNull();
  expect(screen.getByRole('button', { name: 'Jump to source' })).toBeTruthy();
  await userEvent.click(
    screen.getByRole('button', { name: 'Hide source interval' }),
  );
  await waitFor(() =>
    expect(client.createLibraryRedactions).toHaveBeenCalledTimes(2),
  );
  expect(client.createLibraryRedactions).toHaveBeenLastCalledWith({
    recordingId: detail.id,
    ranges: [{ startOffsetMs: 1000, endOffsetMs: 1500, trackIds: ['video-0'] }],
  });
});

test('a late recall read cannot expose an earlier source after navigation', async () => {
  const late = deferred<RecallEvidence | null>();
  routeState.search = { evidenceId: 'earlier', source: 'ocr', seek: 1000 };
  recallClient.resolveRecallEvidence
    .mockReturnValueOnce(late.promise)
    .mockResolvedValue(null);
  const view = render(<RecordingStudioScreen />);
  await screen.findByText('Checking recall source…');
  routeState.search = { evidenceId: 'newer', source: 'ocr', seek: 1000 };
  view.rerender(<RecordingStudioScreen />);
  await screen.findByText(/This recall source is unavailable/);
  await act(async () =>
    late.resolve({
      evidenceId: 'earlier',
      recordingId: detail.id,
      segmentId: 'segment-1',
      sourceKind: 'ocr',
      sourceTrackId: 'video-0',
      sourceId: 'ocr-source',
      sourceFrameRef: 'synthetic-frame',
      documentId: 'document',
      startOffsetMs: 1000,
      endOffsetMs: 1500,
      absoluteStartMs: 2000,
      absoluteEndMs: 2500,
      contentRevision: 'revision',
      text: 'Earlier withheld synthetic source',
    }),
  );
  expect(screen.queryByText('Earlier withheld synthetic source')).toBeNull();
  expect(screen.getByRole('button', { name: 'Check again' })).toBeTruthy();
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
  await mockTimelineRect(timeline);
  await mockPlaybackStartAt(timeline, 12_000);

  await waitFor(() =>
    expect(container.querySelector('video')?.getAttribute('src')).toContain(
      `${longPlaybackAssets.segments[0]!.videoUrl}&startMs=12000&request=1`,
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
  for (const audio of container.querySelectorAll('audio'))
    audio.currentTime = 0.4;
  fireEvent.timeUpdate(video);

  await waitFor(() =>
    expect(
      Number(playbackPosition.getAttribute('aria-valuenow')),
    ).toBeGreaterThan(12_000),
  );
});

test('RecordingStudioScreen inspects accessible activity spans without exposing input contents', async () => {
  debugState.enabled = true;
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
  for (const audio of container.querySelectorAll('audio'))
    audio.currentTime = 6.2;
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
  await act(async () =>
    fireEvent.click(screen.getByRole('button', { name: /^play$/i })),
  );
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
  debugState.enabled = true;
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

test('RecordingStudioScreen steps to actual saved frames with punctuation keys and leaves text editing alone', async () => {
  client.adjacentLibraryFrame
    .mockResolvedValueOnce(500)
    .mockResolvedValueOnce(0);
  render(<RecordingStudioScreen />);
  await screen.findByText('Tracks');
  fireEvent.keyDown(document.body, { code: 'Period', key: '.' });
  await waitFor(() =>
    expect(client.adjacentLibraryFrame).toHaveBeenCalledWith(
      'recording-1',
      0,
      1,
    ),
  );
  await waitFor(() =>
    expect(
      screen
        .getByRole('slider', { name: 'Playback position' })
        .getAttribute('aria-valuenow'),
    ).toBe('500'),
  );
  fireEvent.keyDown(document.body, { code: 'Comma', key: ',' });
  await waitFor(() =>
    expect(client.adjacentLibraryFrame).toHaveBeenLastCalledWith(
      'recording-1',
      500,
      -1,
    ),
  );
  const input = document.createElement('input');
  document.body.append(input);
  fireEvent.keyDown(input, { code: 'Period', key: '.' });
  expect(client.adjacentLibraryFrame).toHaveBeenCalledTimes(2);
  input.remove();
});

test('RecordingStudioScreen distinguishes desktop no-speech from microphone text and clears a cut selected transcript', async () => {
  debugState.enabled = true;
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
  await userEvent.click(await screen.findByRole('tab', { name: 'Details' }));
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
      expect(ocrClient.getOcrTimelineFrames).toHaveBeenCalled(),
    );
    expect(screen.queryByRole('button', { name: /Visual frame/i })).toBeNull();
    expect(aiClient.getRecordingVisualObservations).toHaveBeenCalledWith(
      expect.objectContaining({
        recordingId: detail.id,
        includeOcrRegions: false,
      }),
    );
    if (coarseProgress)
      expect(ocrClient.getOcrTimelineFrames).toHaveBeenCalledWith(
        expect.objectContaining({
          recordingId: detail.id,
          cursor: { id: 'sample-5000', timestampMs: 5_000 },
          limit: 1_000,
        }),
      );
    for (const timestampMs of [0, 5_000, 10_000]) {
      await inspectScreenAt(timestampMs, 38_000);
      await waitFor(() =>
        expect(ocrClient.getOcrDocument).toHaveBeenCalledWith({
          recordingId: detail.id,
          sourceTrackId: 'video-0',
          timestampMs,
          documentId: null,
        }),
      );
      expect(screen.queryByLabelText('Recognized text')).toBeNull();
      await screen.findByText(/Screen recognition complete/);
      expect(screen.queryByText('Identical literal meeting text')).toBeNull();
      expect(
        screen.queryByRole('button', { name: 'Read OCR text' }),
      ).toBeNull();
      expect(
        screen
          .getByRole('slider', { name: 'Playback position' })
          .getAttribute('aria-valuenow'),
      ).toBe(String(timestampMs));
    }
  },
);

test('RecordingStudioScreen withholds an explicitly selected OCR sample after a cut and a stale index read', async () => {
  await loadStudioStyles();
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
  ocrClient.getOcrDocument.mockResolvedValue({
    document: legacyOcrDocument('Synthetic selected OCR', 5000),
    job: null,
    recordingId: detail.id,
    sourceTrackId: 'video-0',
  });
  render(<RecordingStudioScreen />);
  await waitFor(() =>
    expect(ocrClient.getOcrTimelineFrames).toHaveBeenCalled(),
  );
  await inspectScreenAt(5000, 38_000);
  await screen.findByText(/Screen recognition complete/);
  expect(screen.queryByText('Synthetic selected OCR')).toBeNull();
  expect(screen.queryByRole('button', { name: 'Read OCR text' })).toBeNull();
  const stale = deferred<OcrTimelineFramePage>();
  await page.screenshot({ path: '__screenshots__/studio-status-ready.png' });
  ocrClient.getOcrTimelineFrames.mockReturnValue(stale.promise);
  fireEvent.click(screen.getByRole('button', { name: /^Refresh$/i }));
  await waitFor(() =>
    expect(screen.getByText('Loading OCR observation index…')).toBeTruthy(),
  );
  await page.screenshot({ path: '__screenshots__/studio-status-loading.png' });
  ocrClient.getOcrTimelineFrames.mockResolvedValue({
    recordingId: detail.id,
    frames: frames.filter((frame) => frame.frameTimestampMs !== 5_000),
    nextCursor: null,
  });
  const timeline = screen.getByLabelText('Track timeline');
  await mockTimelineRect(timeline);
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
  ocrClient.getOcrDocument.mockResolvedValue({
    document: null,
    job: null,
    recordingId: detail.id,
    sourceTrackId: 'video-0',
  });
  fireEvent.click(screen.getAllByRole('button', { name: /^Redact$/i })[0]!);
  await waitFor(() =>
    expect(client.createLibraryRedactions).toHaveBeenCalled(),
  );
  stale.resolve({ recordingId: detail.id, frames, nextCursor: null });
  await screen.findByRole('group', { name: 'Redacted video' });
  expect(screen.queryByRole('button', { name: 'Read OCR text' })).toBeNull();
  expect(screen.queryByText('Synthetic selected OCR')).toBeNull();
  expect(screen.queryByRole('button', { name: /Visual frame/i })).toBeNull();
});
