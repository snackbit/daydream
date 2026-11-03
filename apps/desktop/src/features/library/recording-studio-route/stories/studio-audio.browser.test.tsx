import { useState } from 'react';
import type * as TauriCore from '@tauri-apps/api/core';
import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  RecordingAiProcessingState,
} from '@daydream/shared';
import {
  cleanup,
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import { RecordingStudio } from '../../components/studio/recording-studio';
import { useStudioTimelineStore } from '../../components/studio/timeline/state/studio-timeline-store';
import { createLibraryPlaybackSession } from '../../library-client';
import syntheticMedia from './fixtures/synthetic-player.webm?url';
import { studioStoryRecording } from './studio-route-fixture';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: native.invoke,
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => undefined),
}));
const detail: LibraryRecordingDetail = {
  ...studioStoryRecording,
  availability: 'available',
  missingReasons: [],
  playableSegmentCount: 1,
  tracks: [
    {
      id: 'video-0',
      kind: 'video',
      label: 'Screen',
      codec: 'h264',
      description: '',
      streamIndex: 0,
    },
    {
      id: 'desktop-audio-1',
      kind: 'desktop-audio',
      label: 'Desktop audio',
      codec: 'opus',
      description: '',
      streamIndex: 1,
    },
    {
      id: 'microphone-2',
      kind: 'microphone',
      label: 'Microphone',
      codec: 'opus',
      description: '',
      streamIndex: 2,
    },
  ],
  segments: [
    {
      id: 'segment',
      durationMs: 10_000,
      endedAtMs: 11_000,
      startedAtMs: 1_000,
      fileExists: true,
      filePath: '/tmp/synthetic-story.mkv',
      pauseAfterMs: 0,
      playbackOffsetMs: 0,
      segmentIndex: 0,
      state: 'finalized',
      videoStreamIndex: 0,
      desktopAudioStreamIndex: 1,
      microphoneAudioStreamIndex: 2,
    },
  ],
};
const assets: LibraryPlaybackAssets = {
  canSeekInStream: true,
  recordingId: detail.id,
  sessionId: 'synthetic-session',
  error: null,
  segments: [
    {
      segmentId: 'segment',
      videoUrl: new URL(syntheticMedia, window.location.origin).href,
      thumbnails: [],
      audio: [
        {
          kind: 'desktop-audio',
          trackId: 'desktop-audio-1',
          streamIndex: 1,
          url: `${new URL(syntheticMedia, window.location.origin).href}?source=desktop`,
        },
        {
          kind: 'microphone',
          trackId: 'microphone-2',
          streamIndex: 2,
          url: `${new URL(syntheticMedia, window.location.origin).href}?source=microphone`,
        },
      ],
    },
  ],
};
const noAction = () => undefined;
const noMutation = async () => undefined;

function renderPlayback(
  debugEnabled = false,
  recordingDetail = detail,
  aiProcessingState: RecordingAiProcessingState | null = null,
) {
  return render(
    <RecordingStudio
      detail={recordingDetail}
      playbackAssets={assets}
      thumbnailStatus="loading"
      debugEnabled={debugEnabled}
      activity={null}
      aiProcessingState={aiProcessingState}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
}

function mediaReady(container: HTMLElement, state: number) {
  for (const media of container.querySelectorAll('video, audio'))
    Object.defineProperty(media, 'readyState', {
      configurable: true,
      get: () => state,
    });
}

test.each([
  ['light', 1440],
  ['dark', 620],
] as const)(
  'desktop audio menu opens reprocessing without seeking or clearing selection in %s at %i pixels',
  async (theme, width) => {
    await page.viewport(width, 960);
    document.documentElement.dataset.theme = theme;
    native.invoke.mockImplementation(async (command: string) => {
      if (command === 'log_frontend_failure') return;
      if (command === 'getRecordingWaveformTiles')
        return { recordingId: detail.id, binMs: 10, tracks: [], error: null };
      if (command === 'previewRecordingReprocessing')
        return {
          recordingId: detail.id,
          planFingerprint: 'synthetic-menu-plan',
          blockedReason: null,
          operations: [
            {
              operation: 'transcription',
              enabled: true,
              remote: false,
              providerLabel: 'Local transcription',
              blockedReason: null,
            },
          ],
          scopes: [
            {
              id: 'desktop-scope',
              trackId: 'desktop-audio-1',
              sourceLabel: 'Desktop audio',
              operation: 'transcription',
              startOffsetMs: 0,
              endOffsetMs: 10000,
            },
          ],
        };
      throw new Error(`Unexpected native command: ${command}`);
    });
    renderPlayback();
    await userEvent.click(
      await screen.findByRole('button', { name: 'Select Desktop audio' }),
    );
    const selected = useStudioTimelineStore.getState().selection;
    const position = screen.getByText('0:00 / 0:10');
    const actions = screen.getByRole('button', {
      name: 'Desktop audio actions',
    });
    await userEvent.click(actions);
    const item = screen.getByRole('menuitem', {
      name: /Reprocess desktop audio/i,
    });
    await page.screenshot({
      path: `__screenshots__/desktop-audio-menu-${theme}.png`,
    });
    await userEvent.click(item, {
      position: { x: item.getBoundingClientRect().width - 8, y: 10 },
    });
    const dialog = await screen.findByRole('dialog', {
      name: 'Reprocess sources',
    });
    await screen.findByText(/Desktop audio · Transcription/);
    expect(useStudioTimelineStore.getState().selection).toBe(selected);
    expect(position).toHaveTextContent('0:00 / 0:10');
    expect(
      native.invoke.mock.calls.filter(
        ([command]) => command === 'previewRecordingReprocessing',
      ),
    ).toEqual([
      [
        'previewRecordingReprocessing',
        {
          input: {
            recordingId: detail.id,
            targets: [{ trackId: 'desktop-audio-1', ranges: null }],
          },
        },
      ],
    ]);
    await page.screenshot({
      element: dialog,
      path: `__screenshots__/desktop-audio-review-${theme}.png`,
    });
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    await userEvent.click(
      screen.getByRole('button', { name: 'Select Desktop audio' }),
      { button: 'right' },
    );
    await userEvent.click(
      screen.getByRole('menuitem', { name: /Reprocess desktop audio/i }),
    );
    await screen.findByRole('dialog', { name: 'Reprocess sources' });
    expect(useStudioTimelineStore.getState().selection).toBe(selected);
    expect(position).toHaveTextContent('0:00 / 0:10');
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    actions.focus();
    await userEvent.keyboard('{ArrowDown}');
    await screen.findByRole('menuitem', { name: /Reprocess desktop audio/i });
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('menu')).toBeNull());
    expect(actions).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}{Enter}');
    await screen.findByRole('dialog', { name: 'Reprocess sources' });
    expect(useStudioTimelineStore.getState().selection).toBe(selected);
    expect(position).toHaveTextContent('0:00 / 0:10');
  },
);
beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  useStudioTimelineStore.setState(
    useStudioTimelineStore.getInitialState(),
    true,
  );
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'log_frontend_failure') return;
    if (command === 'createLibraryPlaybackSession') return assets;
    if (command === 'getRecordingWaveformTiles')
      return { recordingId: detail.id, binMs: 10, tracks: [], error: null };
    throw new Error(`Unexpected native command: ${command}`);
  });
  vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined);
  vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(noAction);
});

function PreparationRetryStudio() {
  const [playbackAssets, setPlaybackAssets] = useState<LibraryPlaybackAssets>({
    ...assets,
    error: 'Synthetic preparation failed',
    segments: [],
  });
  async function retry() {
    setPlaybackAssets(
      await createLibraryPlaybackSession({ recordingId: detail.id }),
    );
  }
  function refresh() {
    void retry();
  }
  return (
    <RecordingStudio
      detail={detail}
      playbackAssets={playbackAssets}
      thumbnailStatus="ready"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={refresh}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />
  );
}

test('STU-14 Retry loading rebuilds failed playback assets through the actual typed client and restores playable controls', async () => {
  render(<PreparationRetryStudio />);
  expect(await screen.findByText('Synthetic preparation failed')).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Retry loading' }));
  expect(await screen.findByRole('button', { name: 'Play' })).toBeTruthy();
  expect(screen.queryByText('Synthetic preparation failed')).toBeNull();
  expect(native.invoke).toHaveBeenCalledWith('createLibraryPlaybackSession', {
    input: { recordingId: detail.id },
  });
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete document.documentElement.dataset.theme;
});

test('immediate Play after seeking queues until video and both audio tracks are ready', async () => {
  const view = renderPlayback();
  mediaReady(view.container, HTMLMediaElement.HAVE_METADATA);
  const position = await screen.findByRole('slider', {
    name: 'Playback position',
  });
  position.focus();
  await userEvent.keyboard('{ArrowRight}');
  mediaReady(view.container, HTMLMediaElement.HAVE_METADATA);
  await userEvent.click(screen.getByRole('button', { name: 'Play' }));
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: 'Pause' })).toBeTruthy();
  const video = view.container.querySelector('video');
  if (!video) throw new Error('Synthetic video missing');
  Object.defineProperty(video, 'readyState', {
    configurable: true,
    value: HTMLMediaElement.HAVE_ENOUGH_DATA,
  });
  fireEvent.canPlay(video);
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
  mediaReady(view.container, HTMLMediaElement.HAVE_ENOUGH_DATA);
  fireEvent.canPlay(video);
  await waitFor(() =>
    expect(HTMLMediaElement.prototype.play).toHaveBeenCalledTimes(3),
  );
  expect(screen.queryByText('Playback failed')).toBeNull();
  expect(position).toHaveAttribute('aria-valuenow', '5000');
});

test('a superseded play rejection cannot abort the new seek', async () => {
  const pending = Promise.withResolvers<void>();
  vi.mocked(HTMLMediaElement.prototype.play).mockImplementationOnce(
    () => pending.promise,
  );
  const view = renderPlayback();
  mediaReady(view.container, HTMLMediaElement.HAVE_ENOUGH_DATA);
  const oldVideo = view.container.querySelector('video');
  if (!oldVideo) throw new Error('Synthetic video missing');
  await userEvent.click(await screen.findByRole('button', { name: 'Play' }));
  fireEvent.play(oldVideo);
  screen.getByRole('slider', { name: 'Playback position' }).focus();
  await userEvent.keyboard('{ArrowRight}');
  mediaReady(view.container, HTMLMediaElement.HAVE_ENOUGH_DATA);
  const replacement = view.container.querySelector('video');
  if (!replacement) throw new Error('Synthetic replacement missing');
  fireEvent.canPlay(replacement);
  await waitFor(() =>
    expect(HTMLMediaElement.prototype.play).toHaveBeenCalledTimes(6),
  );
  await act(async () => {
    pending.reject(new DOMException('Synthetic old start', 'AbortError'));
  });
  expect(screen.queryByText('Playback failed')).toBeNull();
  expect(screen.getByRole('button', { name: 'Pause' })).toBeTruthy();
});

test('Play waits through seeking, rapid replacements ignore old events, and unmount retires pending starts', async () => {
  const view = renderPlayback();
  mediaReady(view.container, HTMLMediaElement.HAVE_ENOUGH_DATA);
  const original = view.container.querySelector('video');
  if (!original) throw new Error('Synthetic video missing');
  let seeking = true;
  Object.defineProperty(original, 'seeking', {
    configurable: true,
    get: () => seeking,
  });
  await userEvent.click(await screen.findByRole('button', { name: 'Play' }));
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
  seeking = false;
  fireEvent.seeked(original);
  await act(async () => undefined);
  screen.getByRole('slider', { name: 'Playback position' }).focus();
  await userEvent.keyboard('{ArrowRight}{ArrowLeft}');
  const current = view.container.querySelector('video');
  expect(current).not.toBe(original);
  fireEvent.error(original);
  fireEvent.pause(original);
  expect(screen.queryByRole('button', { name: 'Retry playback' })).toBeNull();
  const pending = Promise.withResolvers<void>();
  vi.mocked(HTMLMediaElement.prototype.play).mockImplementation(
    () => pending.promise,
  );
  mediaReady(view.container, HTMLMediaElement.HAVE_ENOUGH_DATA);
  if (!current) throw new Error('Synthetic replacement missing');
  fireEvent.canPlay(current);
  vi.mocked(HTMLMediaElement.prototype.pause).mockClear();
  view.unmount();
  expect(HTMLMediaElement.prototype.pause).toHaveBeenCalledTimes(3);
  await act(async () =>
    pending.reject(new DOMException('Synthetic retired start', 'AbortError')),
  );
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === 'log_frontend_failure',
    ),
  ).toBe(false);
});

test.each([
  ['light', 1440],
  ['dark', 1440],
  ['light', 620],
  ['dark', 620],
] as const)(
  'sidebar tabs and speed popup preserve selection in %s at %i pixels',
  async (theme, width) => {
    await page.viewport(width, 960);
    document.documentElement.dataset.theme = theme;
    renderPlayback(true);
    await userEvent.click(
      await screen.findByRole('button', { name: /^Select Desktop audio$/ }),
    );
    if (width < 1200)
      await userEvent.click(
        screen.getByRole('button', { name: 'Review & details' }),
      );
    const inspector = await screen.findByRole('tab', { name: 'Details' });
    expect(inspector).toHaveAttribute('aria-selected', 'true');
    const selected = useStudioTimelineStore.getState().selection;
    await userEvent.click(screen.getByRole('tab', { name: 'Review' }));
    expect(useStudioTimelineStore.getState().selection).toBe(selected);
    await userEvent.click(inspector);
    expect(
      await screen.findByRole('textbox', { name: 'Track name' }),
    ).toBeTruthy();
    await page.screenshot({
      path: `__screenshots__/playback-inspector-${theme}-${width}.png`,
    });
    if (width < 1200)
      await userEvent.click(screen.getByRole('button', { name: 'Close' }));
    await userEvent.click(
      screen.getByRole('combobox', { name: 'Playback speed' }),
    );
    await userEvent.click(screen.getByRole('option', { name: '1.5x' }));
    expect(useStudioTimelineStore.getState().selection).toBe(selected);
    await screen.findByRole('combobox', { name: 'Playback speed' });
    await page.screenshot({
      path: `__screenshots__/playback-speed-${theme}-${width}.png`,
    });
  },
);

test('unavailable recordings select Inspector and disable Teleprompter', async () => {
  renderPlayback(false, {
    ...detail,
    availability: 'missing-files',
    missingReasons: ['Synthetic missing media'],
  });
  expect(await screen.findByRole('tab', { name: 'Review' })).toBeDisabled();
  expect(screen.getByRole('tab', { name: 'Details' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  expect(screen.getByText('Recording details')).toBeTruthy();
  expect(screen.queryByText('Recording ID')).toBeNull();
});

test('Teleprompter and Inspector remain directly accessible with Debug enabled', async () => {
  renderPlayback(true);
  const teleprompter = await screen.findByRole('tab', { name: 'Review' });
  expect(teleprompter).toHaveAttribute('aria-selected', 'true');
  await userEvent.click(screen.getByRole('tab', { name: 'Details' }));
  expect(await screen.findByText('Recording details')).toBeTruthy();
  await userEvent.click(teleprompter);
  expect(teleprompter).toHaveAttribute('aria-selected', 'true');
});

test('Pause cancels queued and pending starts; repeated readiness does not restart playback', async () => {
  const view = renderPlayback();
  mediaReady(view.container, HTMLMediaElement.HAVE_METADATA);
  await userEvent.click(await screen.findByRole('button', { name: 'Play' }));
  await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
  mediaReady(view.container, HTMLMediaElement.HAVE_ENOUGH_DATA);
  const video = view.container.querySelector('video');
  if (!video) throw new Error('Synthetic video missing');
  fireEvent.canPlay(video);
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
  const pending = Promise.withResolvers<void>();
  vi.mocked(HTMLMediaElement.prototype.play).mockImplementationOnce(
    () => pending.promise,
  );
  await userEvent.click(screen.getByRole('button', { name: 'Play' }));
  await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
  await act(async () => pending.resolve());
  expect(screen.getByRole('button', { name: 'Play' })).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Play' }));
  await act(async () => undefined);
  vi.mocked(HTMLMediaElement.prototype.play).mockClear();
  fireEvent.canPlay(video);
  fireEvent.seeked(video);
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: 'Pause' })).toBeTruthy();
});

test('all speed choices apply to every track without replacing media and persist after a seek', async () => {
  const view = renderPlayback();
  const original = view.container.querySelector('video');
  for (const rate of [0.25, 0.5, 1, 1.25, 1.5, 2]) {
    await userEvent.click(
      await screen.findByRole('combobox', { name: 'Playback speed' }),
    );
    await userEvent.click(screen.getByRole('option', { name: `${rate}x` }));
    expect(view.container.querySelector('video')).toBe(original);
    for (const media of view.container.querySelectorAll<HTMLMediaElement>(
      'video, audio',
    )) {
      expect(media.playbackRate).toBe(rate);
      expect(media.preservesPitch).toBe(true);
    }
  }
  (await screen.findByRole('slider', { name: 'Playback position' })).focus();
  await userEvent.keyboard('{ArrowRight}');
  expect(view.container.querySelector('video')).not.toBe(original);
  for (const media of view.container.querySelectorAll<HTMLMediaElement>(
    'video, audio',
  ))
    expect(media.playbackRate).toBe(2);
  await userEvent.click(screen.getByRole('tab', { name: 'Details' }));
  await userEvent.keyboard('{Escape}');
  expect(screen.getByRole('tab', { name: 'Details' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
});

test('a current audio error stops the group, logs safe details and Retry replaces the failed media', async () => {
  const view = renderPlayback();
  mediaReady(view.container, HTMLMediaElement.HAVE_ENOUGH_DATA);
  await userEvent.click(await screen.findByRole('button', { name: 'Play' }));
  const audio = view.container.querySelector('audio');
  if (!audio) throw new Error('Synthetic audio missing');
  fireEvent.error(audio);
  expect(
    await screen.findByRole('button', { name: 'Retry playback' }),
  ).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Play' })).toBeTruthy();
  expect(native.invoke).toHaveBeenCalledWith('log_frontend_failure', {
    input: expect.objectContaining({
      operation: 'studio_playback',
      errorCode: 'playback_media_failed',
    }),
  });
  await userEvent.click(screen.getByRole('button', { name: 'Retry playback' }));
  expect(view.container.querySelector('audio')).not.toBe(audio);
  mediaReady(view.container, HTMLMediaElement.HAVE_ENOUGH_DATA);
  const video = view.container.querySelector('video');
  if (!video) throw new Error('Synthetic replacement missing');
  fireEvent.canPlay(video);
  expect(screen.getByRole('button', { name: 'Pause' })).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Retry playback' })).toBeNull();
});

test('an already failed track rejects Play once and Retry recovers without exposing its decoder message', async () => {
  const view = renderPlayback();
  mediaReady(view.container, HTMLMediaElement.HAVE_ENOUGH_DATA);
  const audio = view.container.querySelector('audio');
  if (!audio) throw new Error('Synthetic audio missing');
  Object.defineProperty(audio, 'error', {
    value: { code: 3, message: 'synthetic-private-decoder-message' },
  });
  await userEvent.click(await screen.findByRole('button', { name: 'Play' }));
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
  fireEvent.error(audio);
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'log_frontend_failure',
    ),
  ).toHaveLength(1);
  expect(screen.queryByText('synthetic-private-decoder-message')).toBeNull();
  await userEvent.click(screen.getByRole('button', { name: 'Retry playback' }));
  mediaReady(view.container, HTMLMediaElement.HAVE_ENOUGH_DATA);
  const video = view.container.querySelector('video');
  if (!video) throw new Error('Synthetic replacement missing');
  fireEvent.canPlay(video);
  expect(screen.getByRole('button', { name: 'Pause' })).toBeTruthy();
});

test('synchronizing an audio track may seek it; the group waits for its resulting seeked event', async () => {
  const view = renderPlayback();
  mediaReady(view.container, HTMLMediaElement.HAVE_ENOUGH_DATA);
  const video = view.container.querySelector('video'),
    audio = view.container.querySelector('audio');
  if (!video || !audio) throw new Error('Synthetic media missing');
  Object.defineProperty(video, 'currentTime', { get: () => 1 });
  let time = 0,
    ready = HTMLMediaElement.HAVE_ENOUGH_DATA as number;
  Object.defineProperty(audio, 'currentTime', {
    get: () => time,
    set: (next) => {
      time = next;
      ready = HTMLMediaElement.HAVE_METADATA;
    },
  });
  Object.defineProperty(audio, 'readyState', {
    configurable: true,
    get: () => ready,
  });
  Object.defineProperty(audio, 'seekable', {
    value: { length: 1, start: () => 0, end: () => 10 },
  });
  await userEvent.click(await screen.findByRole('button', { name: 'Play' }));
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: 'Pause' })).toBeTruthy();
  ready = HTMLMediaElement.HAVE_ENOUGH_DATA;
  fireEvent.seeked(audio);
  await waitFor(() =>
    expect(HTMLMediaElement.prototype.play).toHaveBeenCalledTimes(3),
  );
});

test('real media advances Teleprompter highlights while browsing, changes speed, pauses and resumes after immediate seek/Play', async () => {
  vi.mocked(HTMLMediaElement.prototype.play).mockRestore();
  vi.mocked(HTMLMediaElement.prototype.pause).mockRestore();
  const state: RecordingAiProcessingState = {
    recordingId: detail.id,
    job: null,
    chunks: [],
    transcript: {
      recordingId: detail.id,
      tracks: [
        {
          trackId: 'microphone-2',
          sourceRole: 'microphone',
          segments: [
            {
              id: 'timed-speech',
              segmentId: 'segment',
              startOffsetMs: 0,
              endOffsetMs: 10000,
              provider: 'local-whisper-cpp',
              model: 'synthetic',
              confidence: 1,
              text: 'Early Later',
              words: [
                {
                  id: 'early',
                  text: 'Early',
                  startOffsetMs: 0,
                  endOffsetMs: 2000,
                  probability: 1,
                },
                {
                  id: 'later',
                  text: 'Later',
                  startOffsetMs: 2000,
                  endOffsetMs: 10000,
                  probability: 1,
                },
              ],
            },
          ],
        },
      ],
    },
  };
  const view = renderPlayback(true, detail, state);
  const video = view.container.querySelector('video');
  if (!video) throw new Error('Synthetic video missing');
  await userEvent.click(await screen.findByRole('button', { name: 'Play' }));
  await waitFor(() => {
    expect(video.videoWidth).toBeGreaterThan(0);
    expect(video.currentTime).toBeGreaterThan(0.1);
    for (const audio of view.container.querySelectorAll('audio'))
      expect(audio.currentTime).toBeGreaterThan(0);
  });
  await new Promise<void>((resolve) =>
    video.requestVideoFrameCallback(() => resolve()),
  );
  await waitFor(() =>
    expect(screen.getByText('Early')).toHaveAttribute('aria-current', 'true'),
  );
  fireEvent.wheel(screen.getByRole('list', { name: 'Teleprompter entries' }), {
    deltaY: 1,
  });
  expect(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeTruthy();
  await waitFor(
    () =>
      expect(screen.getByText('Later', { selector: 'span' })).toHaveAttribute(
        'aria-current',
        'true',
      ),
    { timeout: 4000 },
  );
  expect(screen.getByText('Early')).not.toHaveAttribute('aria-current');
  expect(video.paused).toBe(false);
  await userEvent.click(
    screen.getByRole('button', { name: 'Resume auto-scroll' }),
  );
  expect(
    screen.queryByRole('button', { name: 'Resume auto-scroll' }),
  ).toBeNull();
  await userEvent.click(
    screen.getByRole('combobox', { name: 'Playback speed' }),
  );
  await userEvent.click(screen.getByRole('option', { name: '2x' }));
  expect(video.playbackRate).toBeGreaterThan(1.9);
  await userEvent.click(await screen.findByRole('button', { name: 'Pause' }));
  expect(video.paused).toBe(true);
  const pausedTime = video.currentTime;
  await userEvent.click(screen.getByRole('tab', { name: 'Details' }));
  await userEvent.click(screen.getByRole('tab', { name: 'Review' }));
  expect(video.currentTime).toBe(pausedTime);
  screen.getByRole('slider', { name: 'Playback position' }).focus();
  await userEvent.keyboard('{ArrowRight}');
  await userEvent.click(screen.getByRole('button', { name: 'Play' }));
  const replacement = view.container.querySelector('video');
  if (!replacement) throw new Error('Synthetic replacement missing');
  await waitFor(() => expect(replacement.currentTime).toBeGreaterThan(0.1));
  expect(replacement.paused).toBe(false);
  expect(screen.queryByRole('button', { name: 'Retry playback' })).toBeNull();
  await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
});

test('STU-03 adjusting volume and global mute preserves independent source mute while editing the Inspector', async () => {
  const view = render(
    <RecordingStudio
      detail={detail}
      playbackAssets={assets}
      thumbnailStatus="ready"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  await screen.findByRole('button', { name: 'Mute Desktop audio' });
  const desktop = view.container.querySelector<HTMLAudioElement>(
    'audio[src*="desktop"]',
  );
  const microphone = view.container.querySelector<HTMLAudioElement>(
    'audio[src*="microphone"]',
  );
  if (!desktop || !microphone)
    throw new Error('Synthetic source audio elements missing');
  expect(desktop.muted).toBe(false);
  expect(microphone.muted).toBe(false);
  await userEvent.click(
    screen.getByRole('button', { name: 'Mute Desktop audio' }),
  );
  await waitFor(() => expect(desktop.muted).toBe(true));
  expect(microphone.muted).toBe(false);
  await userEvent.click(screen.getByRole('button', { name: 'Mute' }));
  await waitFor(() => expect(microphone.muted).toBe(true));
  await userEvent.click(screen.getByRole('button', { name: 'Unmute' }));
  await waitFor(() => expect(microphone.muted).toBe(false));
  expect(desktop.muted).toBe(true);
  const volume = screen.getByRole('slider', { name: 'Volume' });
  volume.focus();
  await userEvent.keyboard('{Home}{ArrowRight}{ArrowRight}{ArrowRight}');
  await waitFor(() => expect(microphone.volume).toBeCloseTo(0.15));
  expect(desktop.volume).toBeCloseTo(0.15);
  await userEvent.click(
    screen.getByRole('button', { name: 'Select Microphone' }),
  );
  const trackName = await screen.findByRole('textbox', { name: 'Track name' });
  await userEvent.fill(trackName, 'Interview source');
  await userEvent.click(
    screen.getByRole('button', { name: 'Mute Interview source' }),
  );
  await waitFor(() => expect(microphone.muted).toBe(true));
  await userEvent.click(
    screen.getByRole('button', { name: 'Unmute Desktop audio' }),
  );
  await waitFor(() => expect(desktop.muted).toBe(false));
  expect(microphone.muted).toBe(true);
  expect(trackName).toHaveValue('Interview source');
  expect(Number(volume.getAttribute('aria-valuenow'))).toBeCloseTo(0.15);
});

test('STU-05 skip buttons clamp both ends while paused and do not autoplay', async () => {
  render(
    <RecordingStudio
      detail={detail}
      playbackAssets={assets}
      thumbnailStatus="ready"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  const position = await screen.findByRole('slider', {
    name: 'Playback position',
  });
  await userEvent.click(
    screen.getByRole('button', { name: 'Rewind 10 seconds' }),
  );
  expect(position).toHaveAttribute('aria-valuenow', '0');
  await userEvent.click(
    screen.getByRole('button', { name: 'Forward 10 seconds' }),
  );
  expect(position).toHaveAttribute('aria-valuenow', '10000');
  await userEvent.click(
    screen.getByRole('button', { name: 'Forward 10 seconds' }),
  );
  expect(position).toHaveAttribute('aria-valuenow', '10000');
  await userEvent.click(
    screen.getByRole('button', { name: 'Rewind 10 seconds' }),
  );
  expect(position).toHaveAttribute('aria-valuenow', '0');
  expect(screen.getByRole('button', { name: 'Play' })).toBeTruthy();
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
});

test('buffered playback survives a stalled download and genuine buffering resumes when all media is ready', async () => {
  const view = render(
    <RecordingStudio
      detail={detail}
      playbackAssets={assets}
      thumbnailStatus="loading"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  const video = view.container.querySelector('video');
  if (!video) throw new Error('Synthetic video element missing');
  let ready: number = HTMLMediaElement.HAVE_ENOUGH_DATA;
  Object.defineProperty(video, 'readyState', { get: () => ready });
  for (const audio of view.container.querySelectorAll('audio'))
    Object.defineProperty(audio, 'readyState', {
      get: () => HTMLMediaElement.HAVE_ENOUGH_DATA,
    });
  await userEvent.click(await screen.findByRole('button', { name: 'Play' }));
  await screen.findByRole('button', { name: 'Pause' });
  vi.mocked(HTMLMediaElement.prototype.pause).mockClear();
  fireEvent.stalled(video);
  expect(HTMLMediaElement.prototype.pause).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: 'Pause' })).toBeTruthy();

  ready = HTMLMediaElement.HAVE_CURRENT_DATA;
  Object.defineProperty(video, 'buffered', {
    value: { length: 0, start: () => 0, end: () => 0 },
  });
  fireEvent.waiting(video);
  expect(HTMLMediaElement.prototype.pause).toHaveBeenCalled();
  vi.mocked(HTMLMediaElement.prototype.play).mockClear();
  ready = HTMLMediaElement.HAVE_ENOUGH_DATA;
  fireEvent.canPlay(video);
  await waitFor(() =>
    expect(HTMLMediaElement.prototype.play).toHaveBeenCalledTimes(3),
  );
  fireEvent.play(video);
  expect(screen.getByRole('button', { name: 'Pause' })).toBeTruthy();
});

test('seeking during playback resumes the replacement stream while previews are still loading', async () => {
  const view = render(
    <RecordingStudio
      detail={detail}
      playbackAssets={assets}
      thumbnailStatus="loading"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  await userEvent.click(await screen.findByRole('button', { name: 'Play' }));
  await screen.findByRole('button', { name: 'Pause' });
  const previous = view.container.querySelector('video');
  const position = screen.getByRole('slider', { name: 'Playback position' });
  position.focus();
  await userEvent.keyboard('{ArrowRight}');
  const replacement = view.container.querySelector('video');
  if (!replacement) throw new Error('Synthetic replacement stream missing');
  expect(replacement).not.toBe(previous);
  expect(position).toHaveAttribute('aria-valuenow', '5000');
  expect(replacement.getAttribute('src')).toContain('startMs=5000');
  for (const media of view.container.querySelectorAll('video, audio'))
    Object.defineProperty(media, 'readyState', {
      get: () => HTMLMediaElement.HAVE_ENOUGH_DATA,
    });
  vi.mocked(HTMLMediaElement.prototype.play).mockClear();
  fireEvent.canPlay(replacement);
  await waitFor(() =>
    expect(HTMLMediaElement.prototype.play).toHaveBeenCalledTimes(3),
  );
  fireEvent.play(replacement);
  expect(screen.getByRole('button', { name: 'Pause' })).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Retry playback' })).toBeNull();
});

test('STU-06 keyboard reorders a track and cancelling a later drag preserves its committed order', async () => {
  render(
    <RecordingStudio
      detail={detail}
      playbackAssets={assets}
      thumbnailStatus="ready"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  const reorder = await screen.findByRole('button', {
    name: 'Reorder Microphone',
  });
  function labels() {
    return screen
      .getAllByRole('button', { name: /^Select / })
      .map((element) => element.getAttribute('aria-label'));
  }
  const before = labels();
  await userEvent.click(reorder);
  await userEvent.keyboard('{Enter}');
  await screen.findByText(
    /^(Picked up draggable item microphone-2\.|Draggable item microphone-2 was moved over)/,
  );
  await userEvent.keyboard('{ArrowUp}');
  await screen.findByText(
    'Draggable item microphone-2 was moved over droppable area desktop-audio-1.',
  );
  await userEvent.keyboard('{Enter}');
  await waitFor(() => expect(labels()).not.toEqual(before));
  const committed = labels();
  expect(committed.indexOf('Select Microphone')).toBe(
    before.indexOf('Select Microphone') - 1,
  );
  await userEvent.click(reorder);
  await userEvent.keyboard('{Enter}');
  await screen.findByText(
    /^(Picked up draggable item microphone-2\.|Draggable item microphone-2 was moved over)/,
  );
  await userEvent.keyboard('{ArrowUp}{Escape}');
  expect(labels()).toEqual(committed);
  expect(screen.getByRole('button', { name: 'Play' })).toBeTruthy();
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
});

test('STU-12 Skip redacted part reaches its exact eligible boundary without playing paused media', async () => {
  const redacted = {
    ...detail,
    redactions: [
      {
        id: 'video-cut',
        recordingId: detail.id,
        trackIds: ['video-0'],
        startOffsetMs: 0,
        endOffsetMs: 3000,
        createdAtMs: 1,
        reason: null,
      },
    ],
  };
  render(
    <RecordingStudio
      detail={redacted}
      playbackAssets={assets}
      thumbnailStatus="ready"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  await userEvent.click(
    await screen.findByRole('button', { name: 'Skip redacted part' }),
  );
  expect(
    screen.getByRole('slider', { name: 'Playback position' }),
  ).toHaveAttribute('aria-valuenow', '3000');
  expect(screen.queryByLabelText('Redacted video')).toBeNull();
  expect(screen.getByRole('button', { name: 'Play' })).toBeTruthy();
  expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
});

test('STU-13 Retry playback clears the rejected play and resumes the same source after canplay', async () => {
  vi.mocked(HTMLMediaElement.prototype.play).mockRejectedValueOnce(
    new Error('Synthetic playback blocked'),
  );
  const view = render(
    <RecordingStudio
      detail={detail}
      playbackAssets={assets}
      thumbnailStatus="ready"
      activity={null}
      aiProcessingState={null}
      visualObservations={null}
      onRefreshPlayback={noAction}
      onCreateRedactions={noMutation}
      onDeleteRedaction={noMutation}
      onUnredactRanges={noMutation}
    />,
  );
  await userEvent.click(await screen.findByRole('button', { name: 'Play' }));
  expect(
    await screen.findByText('Playback could not start. Try again.'),
  ).toBeTruthy();
  expect(native.invoke).toHaveBeenCalledWith('log_frontend_failure', {
    input: expect.objectContaining({
      operation: 'studio_playback',
      errorCode: 'playback_start_failed',
    }),
  });
  const video = view.container.querySelector('video');
  if (!video) throw new Error('Synthetic video element missing');
  await userEvent.click(screen.getByRole('button', { name: 'Retry playback' }));
  expect(screen.queryByText('Playback could not start. Try again.')).toBeNull();
  const replacement = view.container.querySelector('video');
  if (!replacement) throw new Error('Retry media missing');
  fireEvent.canPlay(replacement);
  fireEvent.play(replacement);
  expect(await screen.findByRole('button', { name: 'Pause' })).toBeTruthy();
  expect(
    screen.getByRole('slider', { name: 'Playback position' }),
  ).toHaveAttribute('aria-valuenow', '0');
  expect(replacement.getAttribute('src')).toContain('synthetic-player.webm');
  expect(video.getAttribute('src')).toBeNull();
});
