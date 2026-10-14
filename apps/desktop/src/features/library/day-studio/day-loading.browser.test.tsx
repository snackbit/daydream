import type { ComponentProps } from 'react';
import type * as TauriCore from '@tauri-apps/api/core';
import type {
  LibraryPlaybackAssets,
  RecallStateChanged,
} from '@daydream/shared';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import {
  recallDayFixture,
  recallStatusFixture,
} from '@/features/recall/testing/recall-browser-fixtures';
import {
  defaultSettingsState,
  scene,
} from '@/features/settings/testing/settings-browser-fixtures';
import '@/index.css';
import { DayScreen } from '@/screens/DayScreen';
import { RecordingDiskUsage } from '../components/studio/inspector/recording-disk-usage';
import { useStudioTimelineStore } from '../components/studio/timeline/state/studio-timeline-store';
import syntheticMedia from '../recording-studio-route/stories/fixtures/synthetic-player.webm?url';
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

// The only substituted boundary is Tauri IPC. Timeline assembly, request
// ownership, sidebar navigation and loading/error rendering remain production.
beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.resetAllMocks();
  manifest = dayTimelineFixture();
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
      args?: { input?: { recordingId?: string }; recordingId?: string },
    ) => {
      const id = args?.input?.recordingId ?? args?.recordingId ?? '';
      const recording = manifest.recordings.find((r) => r.id === id);
      if (command === 'getAppSettings') return defaultSettingsState;
      if (command === 'getRecorderScene') return scene;
      if (command === 'getLibraryDayTimeline') return structuredClone(manifest);
      if (command === 'getLibraryRecording') {
        if (!recording) throw new Error('Unknown synthetic recording');
        return { ...structuredClone(recording), sizeBytes: 4096 };
      }
      if (command === 'createLibraryPlaybackSession') {
        if (!recording) throw new Error('Unknown synthetic recording');
        const url = new URL(syntheticMedia, window.location.origin).href;
        return {
          recordingId: id,
          sessionId: 'session-' + id,
          error: null,
          segments: recording.segments.map((s) => ({
            segmentId: s.id,
            videoUrl: url,
            thumbnails: [],
            videos: [
              {
                trackId: recording.tracks[0]?.id ?? 'video-0',
                streamIndex: 0,
                url,
                thumbnails: [],
              },
            ],
            audio: [],
          })),
        } satisfies LibraryPlaybackAssets;
      }
      if (
        command === 'closeLibraryPlaybackSession' ||
        command === 'log_frontend_failure'
      )
        return;
      if (command === 'getRecordingAiProcessingState')
        return {
          recordingId: id,
          job: null,
          chunks: [],
          transcript: { recordingId: id, tracks: [] },
        };
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
      if (command === 'getRecallDay') return structuredClone(recallDayFixture);
      if (command === 'getRecallStatus') return recallStatusFixture;
      throw new Error('Unexpected synthetic command: ' + command);
    },
  );
});
afterEach(() => {
  cleanup();
  native.handlers.clear();
  delete document.documentElement.dataset.theme;
});
async function openStudio() {
  const view = render(
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <DayScreen date="2026-10-11" />
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

function holdCommand(command: string) {
  const original = native.invoke.getMockImplementation();
  if (!original) throw new Error('Synthetic IPC not initialized');
  let resolve: (value: unknown) => void = () => {
    throw new Error('Deferred IPC not initialized');
  };
  const promise = new Promise<unknown>((done) => {
    resolve = done;
  });
  native.invoke.mockImplementation((name, args) =>
    name === command ? promise : original(name, args),
  );
  return { resolve, restore: () => native.invoke.mockImplementation(original) };
}

test('pending day opening shows the complete scaffold and replaces it with the Studio', async () => {
  const pending = holdCommand('getLibraryDayTimeline');
  const view = render(
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <DayScreen date="2026-10-11" />
    </div>,
  );
  expect(
    screen
      .getByRole('region', { name: 'Loading day studio' })
      .getAttribute('aria-busy'),
  ).toBe('true');
  expect(screen.getByRole('heading', { level: 1 }).textContent).toContain(
    'October 11',
  );
  expect(screen.queryByText('Loading local text and transcripts…')).toBeNull();
  expect(view.container.querySelector('video')).toBeNull();
  expect(
    screen.queryByRole('button', { name: 'Process recording' }),
  ).toBeNull();
  pending.resolve(structuredClone(manifest));
  await screen.findByRole('region', { name: 'Recording player' });
  expect(
    screen.queryByRole('region', { name: 'Loading day studio' }),
  ).toBeNull();
});

test('day failure can retry through the scaffold and reach the genuine empty-day state', async () => {
  const original = native.invoke.getMockImplementation();
  if (!original) throw new Error('Synthetic IPC not initialized');
  native.invoke.mockImplementation((name, args) =>
    name === 'getLibraryDayTimeline'
      ? Promise.reject(new Error('Synthetic day failure'))
      : original(name, args),
  );
  render(<DayScreen date="2026-10-11" />);
  await screen.findByRole('alert');
  const pending = holdCommand('getLibraryDayTimeline');
  await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
  await screen.findByRole('region', { name: 'Loading day studio' });
  pending.resolve({ ...manifest, recordings: [], spans: [] });
  await screen.findByText('No playable recordings for this day.');
  expect(
    screen.queryByRole('region', { name: 'Loading day studio' }),
  ).toBeNull();
});

test('ordinary refresh keeps the populated player visible while the day read is pending', async () => {
  const view = await openStudio();
  const player = view.container.querySelector('video');
  const pending = holdCommand('getLibraryDayTimeline');
  await userEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  expect(view.container.querySelector('video')).toBe(player);
  expect(
    screen.queryByRole('region', { name: 'Loading day studio' }),
  ).toBeNull();
  pending.resolve(structuredClone(manifest));
  await waitFor(() =>
    expect(view.container.querySelector('video')).toBe(player),
  );
});

test('changing days immediately scaffolds the new date and ignores the old response', async () => {
  const old = holdCommand('getLibraryDayTimeline');
  const view = render(<DayScreen date="2026-10-11" />);
  await waitFor(() =>
    expect(native.invoke).toHaveBeenCalledWith(
      'getLibraryDayTimeline',
      expect.anything(),
    ),
  );
  old.restore();
  const next = holdCommand('getLibraryDayTimeline');
  view.rerender(<DayScreen date="2026-10-12" />);
  expect(screen.getByRole('heading', { level: 1 }).textContent).toContain(
    'October 12',
  );
  await act(async () => old.resolve(structuredClone(manifest)));
  expect(
    screen.getByRole('region', { name: 'Loading day studio' }),
  ).toBeTruthy();
  next.resolve({ ...manifest, dayId: '2026-10-12', recordings: [], spans: [] });
  await screen.findByText('No playable recordings for this day.');
  expect(screen.getByRole('heading', { level: 1 }).textContent).toContain(
    'October 12',
  );
});

test('recording disk usage loads only in Details and failures retry without rebuilding playback', async () => {
  for (const recording of manifest.recordings) recording.sizeBytes = null;
  const view = await openStudio();
  const player = view.container.querySelector('video');
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === 'getLibraryRecording',
    ),
  ).toBe(false);
  const original = native.invoke.getMockImplementation();
  if (!original) throw new Error('Synthetic IPC not initialized');
  native.invoke.mockImplementation((name, args) =>
    name === 'getLibraryRecording'
      ? Promise.reject(new Error('Synthetic disk failure'))
      : original(name, args),
  );
  await userEvent.click(screen.getByRole('tab', { name: 'Details' }));
  await screen.findByRole('button', { name: 'Retry disk usage' });
  native.invoke.mockImplementation(original);
  await userEvent.click(
    screen.getByRole('button', { name: 'Retry disk usage' }),
  );
  await screen.findByText('4.10 KB');
  expect(view.container.querySelector('video')).toBe(player);
});

test('privacy invalidation removes details and rejects a late disk-usage result', async () => {
  for (const recording of manifest.recordings) recording.sizeBytes = null;
  const view = await openStudio();
  const pending = holdCommand('getLibraryRecording');
  await userEvent.click(screen.getByRole('tab', { name: 'Details' }));
  await waitFor(() =>
    expect(native.invoke).toHaveBeenCalledWith('getLibraryRecording', {
      recordingId: 'a',
    }),
  );
  const stale = { ...manifest.recordings[0], sizeBytes: 999999 };
  manifest = {
    ...manifest,
    privacyRevision: manifest.privacyRevision + 1,
    recordings: [],
    spans: [],
  };
  act(() => {
    for (const handler of native.handlers)
      handler({
        payload: { dayId: null, privacyRevision: manifest.privacyRevision },
      });
  });
  expect(view.container.querySelector('video')).toBeNull();
  pending.resolve(stale);
  await screen.findByText('No playable recordings for this day.');
  expect(screen.queryByText('1000.0 KB')).toBeNull();
});

test.each([
  { id: 'other' },
  { sizeBytes: null },
  { sizeBytes: 0.5 },
  { sizeBytes: -1 },
])(
  'invalid disk-usage metadata remains unavailable and can be retried: %j',
  async (invalid) => {
    for (const recording of manifest.recordings) recording.sizeBytes = null;
    await openStudio();
    const pending = holdCommand('getLibraryRecording');
    await userEvent.click(screen.getByRole('tab', { name: 'Details' }));
    await waitFor(() =>
      expect(native.invoke).toHaveBeenCalledWith('getLibraryRecording', {
        recordingId: 'a',
      }),
    );
    pending.resolve({ ...manifest.recordings[0], sizeBytes: 100, ...invalid });
    await screen.findByRole('button', { name: 'Retry disk usage' });
    expect(screen.queryByText('100 B')).toBeNull();
  },
);

test('a synthetic day never requests recording disk usage', () => {
  const recording = manifest.recordings[0];
  if (!recording) throw new Error('Synthetic recording missing');
  render(
    <RecordingDiskUsage
      active
      detail={{ ...recording, id: 'day:2026-10-11', sizeBytes: null }}
    />,
  );
  expect(screen.getByText('Select a recording for disk usage')).toBeTruthy();
  expect(native.invoke).not.toHaveBeenCalled();
});

test('a late disk-usage result cannot replace a different recording or create an error after unmount', async () => {
  const first = manifest.recordings[0],
    second = manifest.recordings[1];
  if (!first || !second) throw new Error('Synthetic recordings missing');
  const pending = holdCommand('getLibraryRecording');
  const view = render(
    <RecordingDiskUsage active detail={{ ...first, sizeBytes: null }} />,
  );
  await waitFor(() =>
    expect(native.invoke).toHaveBeenCalledWith('getLibraryRecording', {
      recordingId: first.id,
    }),
  );
  view.rerender(<RecordingDiskUsage active detail={second} />);
  await act(async () => pending.resolve({ ...first, sizeBytes: null }));
  expect(screen.getByText('100 B')).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Retry disk usage' })).toBeNull();
  view.unmount();
});

test('loading scaffold renders in both themes at desktop and narrow widths', async () => {
  const pending = holdCommand('getLibraryDayTimeline');
  const view = render(
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <DayScreen date="2026-10-11" />
    </div>,
  );
  for (const theme of ['light', 'dark']) {
    document.documentElement.dataset.theme = theme;
    for (const width of [1440, 390]) {
      await page.viewport(width, 960);
      await screen.findByRole('region', { name: 'Loading day studio' });
      await page.screenshot({
        path: '__screenshots__/day-loading-' + theme + '-' + width + '.png',
      });
    }
  }
  view.unmount();
  await act(async () => pending.resolve(structuredClone(manifest)));
});
