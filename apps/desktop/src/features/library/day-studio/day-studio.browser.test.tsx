import type { ComponentProps } from 'react';
import type * as TauriCore from '@tauri-apps/api/core';
import type {
  LibraryPlaybackAssets,
  RecallStateChanged,
  CreateLibraryRedactionRangeInput,
  AudioCleanupConfig,
  AudioCleanupState,
} from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  act,
  within,
  renderHook,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import {
  recallDayFixture,
  recallEvidenceFixture,
  recallStatusFixture,
} from '@/features/recall/testing/recall-browser-fixtures';
import {
  defaultSettingsState,
  scene,
} from '@/features/settings/testing/settings-browser-fixtures';
import '@/index.css';
import { DayScreen } from '@/screens/DayScreen';
import { useStudioVideoSources } from '../components/studio/playback/sources/use-studio-video-sources';
import { TRACK_LABEL_WIDTH_PX } from '../components/studio/timeline/interactions/track-timeline-math';
import { useStudioTimelineStore } from '../components/studio/timeline/state/studio-timeline-store';
import syntheticMedia from '../recording-studio-route/stories/fixtures/synthetic-player.webm?url';
import { DayRecordingAudioCleanup } from './day-recording-audio-cleanup';
import type { DayStudioLocation } from './day-studio';
import { dayTimelineFixture } from './day-timeline-fixtures';

const native = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  handlers: new Set<(event: { payload: RecallStateChanged }) => void>(),
}));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: native.invoke,
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: native.listen }));
vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => vi.fn(),
  Link: ({ children, to, ...rest }: { to: string } & ComponentProps<'a'>) => (
    <a href={to} {...rest}>
      {children}
    </a>
  ),
}));
let manifest = dayTimelineFixture();
let review = structuredClone(recallDayFixture);
let cleanupState: AudioCleanupState | null = null;
let failRecording: string | null = null;
let failPreparation = false,
  failResults = false,
  failDelete = false,
  failProcessing = false;
let preparedMedia = false;
const writes: Array<{
  recordingId: string;
  ranges: CreateLibraryRedactionRangeInput[];
}> = [];
function assets(recordingId: string): LibraryPlaybackAssets {
  const recording = manifest.recordings.find((r) => r.id === recordingId);
  if (!recording) throw new Error('Unknown synthetic recording');
  const url = new URL(syntheticMedia, window.location.origin).href;
  return {
    recordingId,
    sessionId: `session-${recordingId}${preparedMedia ? '-prepared' : ''}`,
    error: null,
    segments: recording.segments.map((s) => ({
      canSeekInStream: preparedMedia,
      segmentId: s.id,
      videoUrl: `${url}${preparedMedia ? '?prepared=1' : ''}`,
      thumbnails: [],
      videos: (s.videoStreams ?? []).map((v) => ({
        trackId: v.trackId,
        streamIndex: v.streamIndex,
        url: `${url}?source=${v.trackId}${preparedMedia ? '&prepared=1' : ''}`,
        thumbnails: [],
      })),
      audio: recording.tracks
        .filter((t) => t.kind !== 'video')
        .map((t) => ({
          trackId: t.id,
          kind: t.kind === 'microphone' ? 'microphone' : 'desktop-audio',
          streamIndex: t.streamIndex ?? 1,
          url: `${url}?source=${t.id}`,
        })),
    })),
  };
}
beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  manifest = dayTimelineFixture();
  review = structuredClone(recallDayFixture);
  cleanupState = null;
  failRecording = null;
  failPreparation = false;
  preparedMedia = false;
  failResults = false;
  failDelete = false;
  failProcessing = false;
  writes.length = 0;
  native.handlers.clear();
  useStudioTimelineStore.setState(
    useStudioTimelineStore.getInitialState(),
    true,
  );
  native.listen.mockImplementation(
    async (
      name: string,
      handler: (event: { payload: RecallStateChanged }) => void,
    ) => {
      if (name === 'recall-state-changed') native.handlers.add(handler);
      return () => native.handlers.delete(handler);
    },
  );
  native.invoke.mockImplementation(
    async (
      command: string,
      args?: {
        input?: {
          recordingId?: string;
          ranges?: CreateLibraryRedactionRangeInput[];
          title?: string;
          direction?: number;
          config?: AudioCleanupConfig;
          taskId?: string;
          status?: 'open' | 'done' | 'dismissed';
          reviewed?: boolean;
        };
        recordingId?: string;
      },
    ) => {
      const id = args?.input?.recordingId ?? args?.recordingId ?? '';
      if (command === 'getAppSettings') return defaultSettingsState;
      if (command === 'getRecorderScene') return scene;
      if (command === 'getLibraryDayTimeline') return structuredClone(manifest);
      if (command === 'getAudioCleanup') {
        if (!cleanupState || cleanupState.recordingId !== id)
          cleanupState = {
            recordingId: id,
            config: { mode: 'off', matchThreshold: 0.65, amount: 1 },
            pendingConfig: null,
            revision: null,
            phase: 'idle',
            progress: 0,
            matchedMs: 0,
            referenceAvailable: true,
            transcriptsOutdated: false,
            error: null,
          };
        if (cleanupState.phase === 'running') {
          cleanupState = {
            ...cleanupState,
            config: cleanupState.pendingConfig ?? cleanupState.config,
            pendingConfig: null,
            phase: 'ready',
            revision: 'cleaned-day',
            progress: 1,
            matchedMs: 4000,
          };
          const recording = manifest.recordings.find(
            (recording) => recording.id === id,
          );
          if (recording) recording.audioCleanup = cleanupState;
        }
        return cleanupState;
      }
      if (command === 'applyAudioCleanup') {
        if (!cleanupState || !args?.input?.config)
          throw new Error('Missing synthetic cleanup settings');
        cleanupState = {
          ...cleanupState,
          pendingConfig: args.input.config,
          phase: 'running',
        };
        return cleanupState;
      }
      if (command === 'previewRecordingReprocessing')
        return {
          recordingId: id,
          planFingerprint: 'synthetic-plan',
          blockedReason: null,
          operations: [
            {
              operation: 'transcription',
              enabled: true,
              remote: false,
              providerLabel: 'Local',
              blockedReason: null,
            },
          ],
          scopes: [
            {
              id: 'scope',
              trackId: 'microphone-2',
              sourceLabel: 'Microphone',
              operation: 'transcription',
              startOffsetMs: 0,
              endOffsetMs: 10000,
            },
          ],
        };
      if (command === 'createLibraryPlaybackSession') {
        if (failPreparation)
          return { ...assets(id), error: 'Synthetic preparation failure' };
        return assets(id);
      }
      if (
        command === 'closeLibraryPlaybackSession' ||
        command === 'log_frontend_failure'
      )
        return;
      if (command === 'getRecordingAiProcessingState') {
        if (failResults) throw new Error('Synthetic result read failed');
        return {
          recordingId: id,
          job: null,
          chunks: [],
          transcript: { recordingId: id, tracks: [] },
        };
      }
      if (command === 'getRecordingActivity')
        return {
          recordingId: id,
          events: [],
          intervals: [],
          captureStates: [],
          windowSnapshots: [],
        };
      if (command === 'getRecordingVisualObservations')
        return { recordingId: id, ocrRegions: [], vlmObservations: [] };
      if (command === 'getOcrTimelineFrames')
        return { recordingId: id, frames: [], nextCursor: null };
      if (command === 'getRecordingWaveformTiles')
        return { recordingId: id, binMs: 10, tracks: [], error: null };
      if (command === 'getRecallDay') return structuredClone(review);
      if (command === 'getRecallStatus') return recallStatusFixture;
      if (command === 'updateRecallTask') {
        const task = review.tasks.find(
          (task) => task.id === args?.input?.taskId,
        );
        if (!task || !args?.input?.status)
          throw new Error('Unknown synthetic task');
        task.status = args.input.status;
        return structuredClone(task);
      }
      if (command === 'setRecallDayReviewed') {
        review.day.reviewed = args?.input?.reviewed ?? false;
        return structuredClone(review);
      }
      if (command === 'adjacentLibraryFrame')
        return args?.input?.direction === 1 ? 3300 : null;
      if (command === 'resolveRecallEvidence')
        return {
          ...recallEvidenceFixture,
          recordingId: 'b',
          segmentId: 'b-segment',
          sourceTrackId: 'b-screen-1',
          startOffsetMs: 3000,
          endOffsetMs: 4000,
          absoluteStartMs: 113000,
          absoluteEndMs: 114000,
        };
      if (command === 'renameLibraryRecording') {
        const recording = manifest.recordings.find((r) => r.id === id);
        if (!recording) throw new Error('Unknown recording');
        recording.title = args?.input?.title ?? null;
        return recording;
      }
      if (command === 'deleteLibraryRecording') {
        if (failDelete) throw new Error('Synthetic deletion failure');
        manifest = {
          ...manifest,
          privacyRevision: manifest.privacyRevision + 1,
          recordings: manifest.recordings.filter((r) => r.id !== id),
          spans: manifest.spans.filter((s) => s.recordingId !== id),
        };
        for (const handler of [...native.handlers])
          handler({
            payload: { dayId: null, privacyRevision: manifest.privacyRevision },
          });
        return;
      }
      if (command === 'startRecordingAiProcessing') {
        if (failProcessing) throw new Error('Synthetic processing failure');
        return {
          id: 'job',
          recordingId: id,
          kind: 'ai-processing',
          status: 'queued',
          progress: 0,
          errorCode: null,
          createdAtMs: 1,
          updatedAtMs: 1,
        };
      }
      if (command === 'createLibraryRedactions') {
        const ranges = args?.input?.ranges ?? [];
        writes.push({ recordingId: id, ranges });
        if (id === failRecording) throw new Error('Synthetic storage failure');
        const recording = manifest.recordings.find((r) => r.id === id);
        if (!recording) throw new Error('Unknown recording');
        recording.redactions.push(
          ...ranges.map((range, index) => ({
            ...range,
            id: `${id}-cut-${index}`,
            recordingId: id,
            createdAtMs: 1,
            reason: range.reason ?? null,
          })),
        );
        manifest.privacyRevision++;
        for (const handler of [...native.handlers])
          handler({
            payload: { dayId: null, privacyRevision: manifest.privacyRevision },
          });
        return recording;
      }
      throw new Error(`Unexpected synthetic command: ${command}`);
    },
  );
});
afterEach(() => {
  cleanup();
  native.handlers.clear();
  delete document.documentElement.dataset.theme;
});
async function openStudio(location?: DayStudioLocation) {
  const view = render(
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <DayScreen date="2026-10-11" {...(location ? { location } : {})} />
    </div>,
  );
  await screen.findByRole('region', { name: 'Recording player' });
  await waitFor(() =>
    expect(
      view.container.querySelector('video')?.readyState,
    ).toBeGreaterThanOrEqual(2),
  );
  return view;
}

test('paused Studio defers the next recording decoder until current playback advances', async () => {
  const view = await openStudio();
  const nextLoaded = () =>
    [...view.container.querySelectorAll('video')].some((video) =>
      video.src.includes('source=b-screen-0'),
    );
  expect(nextLoaded()).toBe(false);
  await userEvent.click(screen.getByRole('button', { name: 'Play' }));
  await waitFor(() =>
    expect(view.container.querySelector('video')?.paused).toBe(false),
  );
  await waitFor(() => expect(nextLoaded()).toBe(true));
  await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
  expect(nextLoaded()).toBe(false);
  const media = [...view.container.querySelectorAll('video,audio')];
  view.unmount();
  expect(media.every((element) => !element.hasAttribute('src'))).toBe(true);
});
test.each(['paused', 'playing'])(
  'prepared media upgrades %s playback without autoplay or interrupting intent',
  async (mode) => {
    const view = await openStudio();
    if (mode === 'playing') {
      await userEvent.click(screen.getByRole('button', { name: 'Play' }));
      await waitFor(() =>
        expect(view.container.querySelector('video')?.paused).toBe(false),
      );
    }
    const listener = native.listen.mock.calls.find(
      ([name]) => name === 'library-media-prepared',
    )?.[1];
    if (!listener) throw new Error('Preparation subscription missing');
    const count = native.invoke.mock.calls.filter(
      ([name]) => name === 'createLibraryPlaybackSession',
    ).length;
    preparedMedia = true;
    act(() => listener({ payload: { recordingId: 'a' } }));
    if (mode === 'playing') {
      expect(
        native.invoke.mock.calls.filter(
          ([name]) => name === 'createLibraryPlaybackSession',
        ),
      ).toHaveLength(count);
      await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
    }
    await waitFor(() =>
      expect(view.container.querySelector('video')?.currentSrc).toContain(
        'prepared=1',
      ),
    );
    await waitFor(() =>
      expect(
        view.container.querySelector('video')?.readyState,
      ).toBeGreaterThanOrEqual(2),
    );
    expect(view.container.querySelector('video')?.paused).toBe(true);
    expect(native.invoke).toHaveBeenCalledWith('closeLibraryPlaybackSession', {
      input: { sessionId: 'session-a' },
    });
  },
);

async function openRecordings() {
  const trigger = await screen.findByRole('button', { name: 'Recordings' });
  if (trigger.getAttribute('aria-expanded') !== 'true')
    await userEvent.click(trigger);
}
async function findRecording(name: RegExp) {
  await openRecordings();
  return screen.findByRole('button', { name });
}

test('day microphone actions clean the active recording and reload its playback revision', async () => {
  await openStudio();
  const before = native.invoke.mock.calls.filter(
    ([command, args]) =>
      command === 'createLibraryPlaybackSession' &&
      args?.input?.recordingId === 'a',
  ).length;
  await userEvent.click(
    screen.getByRole('button', { name: 'Microphone actions' }),
  );
  await userEvent.click(
    screen.getByRole('menuitem', { name: 'Remove speaker bleed…' }),
  );
  await screen.findByRole('combobox', { name: 'Cancellation' });
  await userEvent.click(screen.getByRole('combobox', { name: 'Cancellation' }));
  await userEvent.click(screen.getByRole('option', { name: 'Auto' }));
  await userEvent.click(await screen.findByRole('button', { name: 'Apply' }));
  await screen.findByText('Cleaned microphone audio is ready for playback.');
  expect(
    native.invoke.mock.calls.find(
      ([command]) => command === 'applyAudioCleanup',
    )?.[1]?.input.recordingId,
  ).toBe('a');
  expect(
    native.invoke.mock.calls.find(
      ([command]) => command === 'applyAudioCleanup',
    )?.[1]?.input.config.mode,
  ).toBe('auto');
  await waitFor(() =>
    expect(
      native.invoke.mock.calls.filter(
        ([command, args]) =>
          command === 'createLibraryPlaybackSession' &&
          args?.input?.recordingId === 'a',
      ).length,
    ).toBeGreaterThan(before),
  );
});

test('dismissed cleanup stays closed during playback and opens again only from its action', async () => {
  const view = await openStudio();
  await userEvent.click(
    screen.getByRole('button', { name: 'Microphone actions' }),
  );
  await userEvent.click(
    screen.getByRole('menuitem', { name: 'Remove speaker bleed…' }),
  );
  await screen.findByRole('combobox', { name: 'Cancellation' });
  await userEvent.click(
    within(screen.getByRole('dialog'))
      .getAllByRole('button', { name: 'Close' })
      .at(-1)!,
  );
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  const loads = native.invoke.mock.calls.filter(
    ([command]) => command === 'getAudioCleanup',
  ).length;
  await userEvent.click(await screen.findByRole('button', { name: 'Play' }));
  await waitFor(() =>
    expect(view.container.querySelector('video')?.paused).toBe(false),
  );
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'getAudioCleanup',
    ),
  ).toHaveLength(loads);
  await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
  await userEvent.click(
    screen.getByRole('button', { name: 'Microphone actions' }),
  );
  await userEvent.click(
    screen.getByRole('menuitem', { name: 'Remove speaker bleed…' }),
  );
  await screen.findByRole('combobox', { name: 'Cancellation' });
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'getAudioCleanup',
    ),
  ).toHaveLength(loads + 1);
});

test('a missing selected monitor never displays another recording’s redacted monitor', async () => {
  const recording = manifest.recordings[1];
  if (!recording) throw new Error('Missing synthetic recording');
  recording.tracks = recording.tracks.filter(
    (track) => track.id !== 'b-screen-0',
  );
  for (const segment of recording.segments) {
    if (segment.videoStreams)
      segment.videoStreams = segment.videoStreams.filter(
        (stream) => stream.trackId !== 'b-screen-0',
      );
    segment.videoStreamIndex = 1;
  }
  recording.redactions = [
    {
      id: 'hidden-b',
      recordingId: 'b',
      startOffsetMs: 0,
      endOffsetMs: 8000,
      reason: null,
      createdAtMs: 1,
      trackIds: ['b-screen-1'],
    },
  ];
  const view = await openStudio();
  await userEvent.click(screen.getByRole('combobox', { name: 'Video source' }));
  await userEvent.click(screen.getByRole('option', { name: 'Monitor 1' }));
  await userEvent.click(await findRecording(/Recording b/));
  await screen.findByText(/This monitor was not recorded in this segment/);
  expect(view.container.querySelector('video')).toBeNull();
});

test('source mapping distinguishes legacy video from explicit missing mappings and keeps grid provenance', () => {
  const recording = structuredClone(manifest.recordings[0]!);
  const legacy = assets(recording.id);
  for (const segment of legacy.segments) delete segment.videos;
  for (const segment of recording.segments) delete segment.videoStreams;
  const hook = renderHook<
    ReturnType<typeof useStudioVideoSources>,
    { input: LibraryPlaybackAssets | null; selected?: string }
  >(
    ({
      input,
      selected,
    }: {
      input: LibraryPlaybackAssets | null;
      selected?: string;
    }) => useStudioVideoSources(recording, input, selected),
    { initialProps: { input: legacy, selected: 'a-screen-0' } },
  );
  expect(hook.result.current.selectedAssets?.segments[0]?.videoUrl).toBe(
    legacy.segments[0]?.videoUrl,
  );
  hook.rerender({ input: legacy, selected: 'a-screen-1' });
  expect(hook.result.current.selectedAssets?.segments[0]?.videoUrl).toBe('');
  hook.rerender({
    input: {
      ...legacy,
      segments: legacy.segments.map((segment) => ({ ...segment, videos: [] })),
    },
    selected: 'a-screen-0',
  });
  expect(hook.result.current.selectedAssets?.segments[0]?.videoUrl).toBe('');
  expect(hook.result.current.selectedAssets?.segments[0]?.thumbnails).toEqual(
    [],
  );
  hook.rerender({ input: null, selected: 'unknown-monitor' });
  expect(hook.result.current.selectedAssets).toBeNull();
  act(() => hook.result.current.select('unknown-monitor'));
  expect(hook.result.current.selectedTrackId).toBe('a-screen-0');
  hook.unmount();

  const dayRecording = manifest.recordings[0]!;
  const grid = renderHook(() =>
    useStudioVideoSources(dayRecording, assets(dayRecording.id)),
  );
  act(() => grid.result.current.toggleGrid());
  expect(grid.result.current.grid).toBe(false);
  act(() => grid.result.current.toggleGrid());
  expect(grid.result.current.grid).toBe(true);
  useStudioTimelineStore.getState().setSelection({
    ...useStudioTimelineStore.getState().selection,
    visualFrameIds: ['synthetic-frame'],
  });
  act(() => grid.result.current.select('a-screen-1'));
  expect(grid.result.current.selectedDetail.segments[0]?.videoTrackId).toBe(
    'a-screen-1',
  );
  expect(grid.result.current.selectedDetail.segments[0]?.videoStreamIndex).toBe(
    1,
  );
  expect(grid.result.current.selectedAssets?.segments[0]?.videoUrl).toContain(
    'source=a-screen-1',
  );
  expect(useStudioTimelineStore.getState().selection.visualFrameIds).toEqual(
    [],
  );
});

test.each(['completed', 'recording', 'paused'] as const)(
  'cleanup retains its explicit request and recording scope when %s',
  async (status) => {
    const detail = { ...manifest.recordings[0]!, status, title: null };
    cleanupState = {
      recordingId: detail.id,
      config: { mode: 'auto', matchThreshold: 0.65, amount: 1 },
      pendingConfig: null,
      revision: 'cleaned',
      phase: 'ready',
      progress: 1,
      matchedMs: 1000,
      referenceAvailable: true,
      transcriptsOutdated: true,
      error: null,
    };
    const pause = vi.fn();
    const readRange = () => ({ startOffsetMs: 0, endOffsetMs: 1000 });
    const refresh = vi.fn();
    const view = render(
      <DayRecordingAudioCleanup
        detail={detail}
        requestRevision={1}
        readRange={readRange}
        pausePlayback={pause}
        refreshPlayback={refresh}
      />,
    );
    await screen.findByRole('combobox', { name: 'Cancellation' });
    view.rerender(
      <DayRecordingAudioCleanup
        detail={detail}
        requestRevision={1}
        readRange={readRange}
        pausePlayback={vi.fn()}
        refreshPlayback={refresh}
      />,
    );
    expect(pause).toHaveBeenCalledTimes(1);
    expect(
      native.invoke.mock.calls.filter(
        ([command]) => command === 'getAudioCleanup',
      ),
    ).toHaveLength(1);
    if (status === 'completed') {
      await userEvent.click(
        screen.getByRole('button', {
          name: 'Regenerate microphone transcripts',
        }),
      );
      await screen.findByRole('dialog', { name: 'Reprocess sources' });
      expect(native.invoke).toHaveBeenCalledWith(
        'previewRecordingReprocessing',
        {
          input: {
            recordingId: 'a',
            targets: [{ trackId: 'microphone-2', ranges: null }],
            operations: ['transcription'],
          },
        },
      );
    } else {
      expect(screen.getByRole('button', { name: 'Apply' })).toBeDisabled();
      expect(
        screen.getByText('Finish recording before removing speaker bleed.'),
      ).toBeTruthy();
    }
  },
);

test.each(['light', 'dark'] as const)(
  'day opens Studio, integrates recordings and Day content, and seeks across records in %s',
  async (theme) => {
    document.documentElement.dataset.theme = theme;
    const view = await openStudio();
    expect(screen.getByRole('tab', { name: 'Review' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Recordings' }));
    expect(screen.getByRole('button', { name: /Recording a/ })).toHaveAttribute(
      'aria-current',
      'true',
    );
    expect(screen.getByRole('img', { name: /Not recorded/ })).toBeTruthy();
    expect(screen.getByLabelText('Monitor 2 synchronized video')).toBeTruthy();
    await userEvent.click(screen.getByRole('button', { name: /Recording b/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Recordings' }));
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: /Recording b/ }),
      ).toHaveAttribute('aria-current', 'true'),
    );
    expect(
      view.container.querySelector('video')?.getAttribute('src'),
    ).toContain('source=b-screen-0');
    expect(view.container.querySelector('video')?.paused).toBe(true);
    await userEvent.click(screen.getByRole('button', { name: 'Day review' }));
    await screen.findByText(recallDayFixture.day.summary ?? '');
    await page.screenshot({ path: `__screenshots__/day-studio-${theme}.png` });
  },
);

test.each([
  { theme: 'light', width: 1440 },
  { theme: 'dark', width: 1440 },
  { theme: 'light', width: 520 },
  { theme: 'dark', width: 520 },
])(
  'Day review uses the wider pane and preserves focus at $width in $theme',
  async ({ theme, width }) => {
    document.documentElement.dataset.theme = theme;
    await page.viewport(width, 960);
    await openStudio();
    const trigger = screen.getByRole('button', { name: 'Day review' });
    await userEvent.click(trigger);
    const dialog = await screen.findByRole('dialog', { name: 'Day review' });
    await within(dialog).findByRole('button', { name: 'Generate summary' });
    await waitFor(() => {
      const bounds = dialog.getBoundingClientRect();
      expect(bounds.width).toBe(width === 1440 ? 896 : width);
      expect(bounds.right).toBeLessThanOrEqual(width);
    });
    await userEvent.click(
      within(dialog).getByRole('button', {
        name: 'Inspect coverage intervals',
      }),
    );
    expect(
      within(dialog).queryByRole('button', {
        name: 'Recorded text and transcripts',
      }),
    ).toBeNull();
    await page.screenshot({
      path: `__screenshots__/day-review-${theme}-${width}.png`,
    });
    await userEvent.keyboard('{Escape}');
    await waitFor(() =>
      expect(screen.queryByRole('dialog', { name: 'Day review' })).toBeNull(),
    );
    expect(document.activeElement).toBe(trigger);
  },
);

test('real media progresses at the selected speed across all monitor and audio streams', async () => {
  const view = await openStudio();
  await userEvent.click(screen.getByRole('button', { name: 'Play' }));
  const video = view.container.querySelector('video');
  if (!video) throw new Error('Video missing');
  await waitFor(() => expect(video.currentTime).toBeGreaterThan(0.25));
  async function sample(rate: number) {
    await waitFor(() => {
      for (const element of view.container.querySelectorAll('video,audio')) {
        if (
          !(element instanceof HTMLMediaElement) ||
          element.closest('[hidden]')
        )
          continue;
        expect(element.playbackRate).toBeCloseTo(rate, 1);
        expect(element.seeking).toBe(false);
      }
    });
    const wall = performance.now(),
      start = video!.currentTime;
    await waitFor(
      () => expect(video!.currentTime - start).toBeGreaterThan(1.2),
      { timeout: 3000 },
    );
    const elapsed = (performance.now() - wall) / 1000;
    expect((video!.currentTime - start) / elapsed).toBeGreaterThan(rate * 0.8);
    expect((video!.currentTime - start) / elapsed).toBeLessThan(rate * 1.2);
  }
  await sample(1);
  await userEvent.click(
    screen.getByRole('combobox', { name: 'Playback speed' }),
  );
  await userEvent.click(screen.getByRole('option', { name: '2x' }));
  await sample(2);
  await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
  const current = view.container.querySelectorAll('video,audio');
  for (const element of current) {
    if (
      element instanceof HTMLMediaElement &&
      element.closest('[hidden]') === null
    ) {
      expect(element.paused).toBe(true);
      expect(element.playbackRate).toBeGreaterThan(1.9);
    }
  }
});

test('recording-local deep links select the owning day clip and keyboard frame steps keep source offsets', async () => {
  const view = await openStudio({
    recordingId: 'b',
    seek: 3000,
    evidenceId: null,
    source: null,
  });
  await waitFor(() =>
    expect(
      view.container.querySelector('video')?.getAttribute('src'),
    ).toContain('startMs=3000'),
  );
  await openRecordings();
  expect(screen.getByRole('button', { name: /Recording b/ })).toHaveAttribute(
    'aria-current',
    'true',
  );
  await userEvent.keyboard('{Escape}');
  await waitFor(() =>
    expect(document.activeElement).toBe(
      screen.getByRole('button', { name: 'Recordings' }),
    ),
  );
  await userEvent.keyboard('.');
  await waitFor(() =>
    expect(native.invoke).toHaveBeenCalledWith('adjacentLibraryFrame', {
      input: {
        recordingId: 'b',
        sourceTrackId: 'b-screen-0',
        timestampMs: 3000,
        direction: 1,
      },
    }),
  );
  await waitFor(() =>
    expect(
      view.container.querySelector('video')?.getAttribute('src'),
    ).toContain('startMs=3300'),
  );
  await userEvent.keyboard(',');
  await waitFor(() =>
    expect(native.invoke).toHaveBeenCalledWith('adjacentLibraryFrame', {
      input: {
        recordingId: 'b',
        sourceTrackId: 'b-screen-0',
        timestampMs: 3300,
        direction: -1,
      },
    }),
  );
});

test('native evidence links select their monitor and preserve recording-local source identity', async () => {
  const view = await openStudio({
    recordingId: 'b',
    seek: null,
    evidenceId: 'evidence-1',
    source: 'ocr',
  });
  await waitFor(() =>
    expect(
      view.container.querySelector('video')?.getAttribute('src'),
    ).toContain('source=b-screen-1'),
  );
  await waitFor(() =>
    expect(
      view.container.querySelector('video')?.getAttribute('src'),
    ).toContain('startMs=3000'),
  );
  expect(screen.getByRole('tab', { name: 'Details' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  expect(
    native.invoke.mock.calls.some(
      ([command, args]) =>
        command === 'resolveRecallEvidence' &&
        args.input.evidenceId === 'evidence-1',
    ),
  ).toBe(true);
});

test('renaming a recording keeps its media buffers and processing actions stay scoped to that recording', async () => {
  const view = await openStudio(),
    video = view.container.querySelector('video');
  await userEvent.click(screen.getByRole('button', { name: 'Rename' }));
  const field = screen.getByRole('textbox');
  await userEvent.clear(field);
  await userEvent.type(field, 'Renamed recording');
  await userEvent.click(screen.getByRole('button', { name: 'Save' }));
  await findRecording(/Renamed recording/);
  await userEvent.keyboard('{Escape}');
  expect(view.container.querySelector('video')).toBe(video);
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'createLibraryPlaybackSession',
    ),
  ).toHaveLength(2);
  failProcessing = true;
  await userEvent.click(
    screen.getByRole('button', { name: 'Process recording' }),
  );
  await screen.findByText('Processing could not start. Try again.');
  expect(native.invoke).toHaveBeenCalledWith('startRecordingAiProcessing', {
    input: { recordingId: 'a' },
  });
  failProcessing = false;
  await userEvent.click(
    screen.getByRole('button', { name: 'Process recording' }),
  );
  await waitFor(() =>
    expect(
      screen.queryByText('Processing could not start. Try again.'),
    ).toBeNull(),
  );
});

test('failed deletion releases the player and a subsequent delete keeps the next recording in Studio', async () => {
  await openStudio();
  failDelete = true;
  await userEvent.click(screen.getByRole('button', { name: 'Delete' }));
  await userEvent.click(
    within(screen.getByRole('dialog')).getByRole('button', { name: 'Delete' }),
  );
  await screen.findByRole('alert');
  await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(screen.queryByText('Updating recording privacy…')).toBeNull();
  failDelete = false;
  await userEvent.click(screen.getByRole('button', { name: 'Delete' }));
  await userEvent.click(
    within(screen.getByRole('dialog')).getByRole('button', { name: 'Delete' }),
  );
  await waitFor(() =>
    expect(screen.queryByRole('button', { name: /Recording a/ })).toBeNull(),
  );
  await findRecording(/Recording b/);
});

test('playback preparation and derived reads recover on Refresh without requiring a metadata change', async () => {
  failPreparation = true;
  failResults = true;
  const view = render(
    <div style={{ height: '100vh' }}>
      <DayScreen date="2026-10-11" />
    </div>,
  );
  await screen.findByText(
    'Playback could not be prepared. Try refreshing the day.',
  );
  failPreparation = false;
  failResults = false;
  await userEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() =>
    expect(
      view.container.querySelector('video')?.readyState,
    ).toBeGreaterThanOrEqual(2),
  );
  await waitFor(() =>
    expect(
      screen.queryByText(
        'Playback could not be prepared. Try refreshing the day.',
      ),
    ).toBeNull(),
  );
  expect(native.invoke).toHaveBeenCalledWith('closeLibraryPlaybackSession', {
    input: { sessionId: 'session-a' },
  });
});

test.each([false, true])(
  'preloads the next recording and pauses its gap countdown without playing it (indexed: %s)',
  async (indexed) => {
    preparedMedia = indexed;
    const view = await openStudio();
    const active = view.container.querySelector('video');
    if (!active) throw new Error('Video missing');
    await userEvent.click(screen.getByRole('button', { name: 'Play' }));
    await waitFor(() => expect(active.paused).toBe(false));
    await waitFor(() =>
      expect(view.container.querySelectorAll('video').length).toBeGreaterThan(
        2,
      ),
    );
    fireEvent.ended(active);
    await screen.findByText(/Not recorded. Next recording at/);
    await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
    const countdown = screen.getByText(/Skipping in/).textContent;
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 1100));
    });
    expect(screen.getByText(/Skipping in/).textContent).toBe(countdown);
    const prepared = view.container.querySelector('div[hidden] video');
    expect(prepared).toBeTruthy();
    await userEvent.click(screen.getByRole('button', { name: 'Play now' }));
    await openRecordings();
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: /Recording b/ }),
      ).toHaveAttribute('aria-current', 'true'),
    );
    expect(view.container.querySelector('video')).toBe(prepared);
    if (!(prepared instanceof HTMLVideoElement))
      throw new Error('Prepared video missing');
    expect(prepared.paused).toBe(true);
  },
);

test('privacy invalidation immediately removes player content and rejects late preparation', async () => {
  const view = await openStudio();
  const old = manifest.privacyRevision;
  manifest = {
    ...manifest,
    privacyRevision: old + 1,
    spans: [],
    recordings: [],
  };
  act(() => {
    for (const handler of native.handlers)
      handler({ payload: { dayId: null, privacyRevision: old + 1 } });
  });
  expect(view.container.querySelector('video')).toBeNull();
  await screen.findByText('No playable recordings for this day.');
  await waitFor(() =>
    expect(native.invoke).toHaveBeenCalledWith('closeLibraryPlaybackSession', {
      input: { sessionId: 'session-a' },
    }),
  );
});

test('cross-recording redaction retains a failed source write through privacy remounts and retries only that portion', async () => {
  const view = await openStudio();
  failRecording = 'b';
  await new Promise<void>((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
  );
  const timeline = screen.getByRole('group', { name: 'Track timeline' }),
    lane = timeline.querySelector(
      '[data-lane-content][data-track-id="day-video-source-monitor-0"]',
    );
  if (!(lane instanceof HTMLElement)) throw new Error('Video lane missing');
  const rect = timeline.getBoundingClientRect(),
    width = rect.width - TRACK_LABEL_WIDTH_PX,
    y = lane.getBoundingClientRect().top + 20;
  const x = (ms: number) =>
    rect.left + TRACK_LABEL_WIDTH_PX + (ms / 18000) * width;
  fireEvent.pointerDown(lane, {
    button: 0,
    pointerId: 42,
    clientX: x(4000),
    clientY: y,
  });
  fireEvent.pointerMove(lane, {
    button: 0,
    pointerId: 42,
    clientX: x(12000),
    clientY: y,
  });
  fireEvent.pointerUp(lane, {
    button: 0,
    pointerId: 42,
    clientX: x(12000),
    clientY: y,
  });
  const redact = screen.getAllByRole('button', { name: /^Redact$/ })[0];
  if (!redact) throw new Error('Redact action missing');
  await userEvent.click(redact);
  await screen.findByRole('button', { name: 'Retry remaining changes' });
  expect(writes.map((w) => w.recordingId)).toEqual(['a', 'b']);
  expect(writes[0]?.ranges[0]).toMatchObject({
    startOffsetMs: 25000,
    endOffsetMs: 26000,
    trackIds: ['a-screen-0'],
  });
  expect(writes[1]?.ranges[0]).toMatchObject({
    startOffsetMs: 0,
    endOffsetMs: 2000,
    trackIds: ['b-screen-0'],
  });
  expect(manifest.recordings[0]?.redactions).toHaveLength(1);
  expect(manifest.recordings[1]?.redactions).toHaveLength(0);
  failRecording = null;
  await userEvent.click(
    screen.getByRole('button', { name: 'Retry remaining changes' }),
  );
  await waitFor(() =>
    expect(
      screen.queryByRole('button', { name: 'Retry remaining changes' }),
    ).toBeNull(),
  );
  expect(writes.map((w) => w.recordingId)).toEqual(['a', 'b', 'b']);
  expect(manifest.recordings[1]?.redactions).toHaveLength(1);
  await waitFor(() =>
    expect(view.container.querySelector('video')).toBeTruthy(),
  );
});

test('empty days offer capture and narrow Studio preserves monitor and recording controls', async () => {
  manifest = { ...manifest, recordings: [], spans: [] };
  review.tasks = [
    {
      id: 'saved-task',
      dayId: '2026-10-11',
      title: 'Saved local task',
      notes: '',
      status: 'open',
      sourceWithheld: false,
      citations: [],
      updatedAtMs: 1,
    },
  ];
  const empty = render(
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <DayScreen date="2026-10-11" />
    </div>,
  );
  await screen.findByRole('link', { name: 'Choose sources' });
  await screen.findByText(recallDayFixture.day.summary ?? '');
  expect(screen.getByRole('button', { name: 'Mark reviewed' })).toBeTruthy();
  await screen.findByText('Saved local task');
  await userEvent.click(screen.getByRole('button', { name: 'Mark done' }));
  await screen.findByRole('button', { name: 'Reopen' });
  expect(review.tasks[0]?.status).toBe('done');
  await userEvent.click(screen.getByRole('button', { name: 'Mark reviewed' }));
  await screen.findByRole('button', { name: 'Mark unreviewed' });
  expect(review.day.reviewed).toBe(true);
  await page.screenshot({
    path: '__screenshots__/day-review-without-media.png',
  });
  document.documentElement.dataset.theme = 'dark';
  await page.viewport(390, 900);
  await userEvent.click(screen.getByRole('button', { name: 'Reopen' }));
  await screen.findByRole('button', { name: 'Mark done' });
  expect(review.tasks[0]?.status).toBe('open');
  await page.screenshot({
    path: '__screenshots__/day-review-without-media-dark-narrow.png',
  });
  empty.unmount();
  delete document.documentElement.dataset.theme;
  manifest = dayTimelineFixture();
  await page.viewport(800, 900);
  await openStudio();
  await userEvent.click(
    screen.getByRole('button', { name: 'Review & details' }),
  );
  await screen.findByRole('tab', { name: 'Review' });
  expect(screen.getByRole('combobox', { name: 'Video source' })).toBeTruthy();
  await page.screenshot({ path: '__screenshots__/day-studio-narrow.png' });
});
