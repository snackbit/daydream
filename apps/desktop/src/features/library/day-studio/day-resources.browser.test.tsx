import type * as TauriCore from '@tauri-apps/api/core';
import type { LibraryPlaybackAssets } from '@daydream/shared';
import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { buildDayTimeline } from './day-timeline';
import { dayTimelineFixture } from './day-timeline-fixtures';
import { useDayPlaybackSessions } from './use-day-playback-sessions';

const native = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: native.listen }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: native.invoke,
}));
function assets(id: string): LibraryPlaybackAssets {
  return {
    recordingId: id,
    sessionId: `session-${id}`,
    error: null,
    segments: [
      {
        segmentId: `${id}-segment`,
        videoUrl: 'http://localhost/synthetic',
        audio: [],
        thumbnails: [],
      },
    ],
  };
}
beforeEach(() => {
  vi.resetAllMocks();
  native.listen.mockResolvedValue(() => undefined);
  native.invoke.mockImplementation(
    async (command: string, args: { input: { recordingId: string } }) => {
      if (command === 'createLibraryPlaybackSession')
        return assets(args.input.recordingId);
      if (
        command === 'closeLibraryPlaybackSession' ||
        command === 'log_frontend_failure'
      )
        return;
      throw new Error('Unexpected native command');
    },
  );
});
afterEach(cleanup);
test('a newer upgrade intent never publishes the candidate retired by an earlier intent', async () => {
  const model = buildDayTimeline(dayTimelineFixture());
  const view = renderHook(() => useDayPlaybackSessions(model));
  await act(async () => view.result.current.ensure('a'));
  expect(native.invoke).toHaveBeenCalledWith('createLibraryPlaybackSession', {
    input: { recordingId: 'a', lazyThumbnails: true, includeRuns: false },
  });
  let release: (value: LibraryPlaybackAssets) => void = () => {
    throw new Error('Upgrade not requested');
  };
  let requests = 0;
  native.invoke.mockImplementation(async (command: string) => {
    if (command !== 'createLibraryPlaybackSession') return;
    requests++;
    if (requests === 1)
      return new Promise<LibraryPlaybackAssets>((resolve) => {
        release = resolve;
      });
    return { ...assets('a'), sessionId: 'newer-candidate' };
  });
  let earlier: Promise<LibraryPlaybackAssets>,
    newer: Promise<LibraryPlaybackAssets>;
  act(() => {
    earlier = view.result.current.ensure('a', true, () => false);
    newer = view.result.current.ensure('a', true, () => true);
  });
  await act(async () => {
    release({ ...assets('a'), sessionId: 'retired-candidate' });
    await Promise.all([earlier, newer]);
  });
  expect(view.result.current.sessions.get('a')?.sessionId).toBe(
    'newer-candidate',
  );
  expect(native.invoke).toHaveBeenCalledWith('closeLibraryPlaybackSession', {
    input: { sessionId: 'retired-candidate' },
  });
  expect(native.invoke).not.toHaveBeenCalledWith(
    'closeLibraryPlaybackSession',
    { input: { sessionId: 'newer-candidate' } },
  );
});
test('prepared-session upgrades defer to newer playback intent and retire the old session only after acceptance', async () => {
  const model = buildDayTimeline(dayTimelineFixture());
  const view = renderHook(() => useDayPlaybackSessions(model));
  await act(async () => view.result.current.ensure('a'));
  const old = view.result.current.sessions.get('a');
  const listener = native.listen.mock.calls[0]?.[1];
  if (!listener) throw new Error('Preparation listener missing');
  act(() => listener({ payload: { recordingId: 'a' } }));
  expect(view.result.current.preparedIds.has('a')).toBe(true);
  native.invoke.mockImplementation(async (command: string) =>
    command === 'createLibraryPlaybackSession'
      ? {
          ...assets('a'),
          sessionId: 'prepared-a',
          segments: assets('a').segments.map((segment) => ({
            ...segment,
            canSeekInStream: true,
          })),
        }
      : undefined,
  );
  await act(async () => view.result.current.ensure('a', true, () => false));
  expect(view.result.current.sessions.get('a')).toBe(old);
  expect(view.result.current.preparedIds.has('a')).toBe(true);
  native.invoke.mockClear();
  await act(async () => view.result.current.ensure('a', true, () => true));
  expect(view.result.current.sessions.get('a')?.sessionId).toBe('prepared-a');
  expect(view.result.current.preparedIds.has('a')).toBe(false);
  expect(native.invoke).toHaveBeenCalledWith('closeLibraryPlaybackSession', {
    input: { sessionId: 'session-a' },
  });
});
test('recording admission shares pending preparation, retains sessions through renames, and retires removed records', async () => {
  const fixture = dayTimelineFixture(),
    model = buildDayTimeline(fixture);
  let complete: (assets: LibraryPlaybackAssets) => void = () => {
    throw new Error('Preparation not started');
  };
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'createLibraryPlaybackSession')
      return new Promise<LibraryPlaybackAssets>((resolve) => {
        complete = resolve;
      });
    if (
      command === 'closeLibraryPlaybackSession' ||
      command === 'log_frontend_failure'
    )
      return;
    throw new Error('Unexpected command');
  });
  const view = renderHook(({ model }) => useDayPlaybackSessions(model), {
    initialProps: { model },
  });
  let first: Promise<LibraryPlaybackAssets>,
    second: Promise<LibraryPlaybackAssets>;
  act(() => {
    first = view.result.current.ensure('a');
    second = view.result.current.ensure('a');
  });
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'createLibraryPlaybackSession',
    ),
  ).toHaveLength(1);
  await act(async () => {
    complete(assets('a'));
    await Promise.all([first, second]);
  });
  expect(view.result.current.sessions.has('a')).toBe(true);
  fixture.recordings[0]!.title = 'Renamed';
  view.rerender({ model: buildDayTimeline(fixture) });
  await act(async () => {
    await view.result.current.ensure('a');
  });
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'createLibraryPlaybackSession',
    ),
  ).toHaveLength(1);
  const removed = {
    ...fixture,
    recordings: fixture.recordings.slice(1),
    spans: fixture.spans.slice(1),
  };
  view.rerender({ model: buildDayTimeline(removed) });
  await waitFor(() =>
    expect(view.result.current.sessions.has('a')).toBe(false),
  );
  await expect(view.result.current.ensure('a')).rejects.toThrow();
  expect(native.invoke).toHaveBeenCalledWith('closeLibraryPlaybackSession', {
    input: { sessionId: 'session-a' },
  });
});
test('late duplicate consumers cannot publish or repeatedly close a retired session', async () => {
  const model = buildDayTimeline(dayTimelineFixture());
  let complete: (assets: LibraryPlaybackAssets) => void = () => {};
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'createLibraryPlaybackSession')
      return new Promise<LibraryPlaybackAssets>((resolve) => {
        complete = resolve;
      });
    if (
      command === 'closeLibraryPlaybackSession' ||
      command === 'log_frontend_failure'
    )
      return;
    throw new Error('Unexpected native command');
  });
  const view = renderHook(() => useDayPlaybackSessions(model));
  const first = view.result.current.ensure('a').catch((error) => error),
    second = view.result.current.ensure('a').catch((error) => error);
  view.unmount();
  complete(assets('a'));
  await Promise.all([first, second]);
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === 'closeLibraryPlaybackSession',
    ),
  ).toHaveLength(1);
});
test.each(['failed', 'empty', 'wrong recording', 'rejected'] as const)(
  'preparation reports %s and admits a later successful recovery',
  async (mode) => {
    const model = buildDayTimeline(dayTimelineFixture());
    let fail = true;
    native.invoke.mockImplementation(async (command: string) => {
      if (
        command === 'closeLibraryPlaybackSession' ||
        command === 'log_frontend_failure'
      )
        return;
      if (command !== 'createLibraryPlaybackSession')
        throw new Error('Unexpected native command');
      if (!fail) return assets('a');
      if (mode === 'rejected') throw new Error('Synthetic native failure');
      const value = assets('a');
      if (mode === 'failed') value.error = 'Synthetic failure';
      if (mode === 'empty') value.segments = [];
      if (mode === 'wrong recording') value.recordingId = 'b';
      return value;
    });
    const view = renderHook(() => useDayPlaybackSessions(model));
    await act(async () => {
      await expect(view.result.current.ensure('a')).rejects.toThrow();
    });
    expect(view.result.current.error).toBeTruthy();
    expect(view.result.current.sessions.size).toBe(0);
    fail = false;
    await act(async () => {
      await view.result.current.ensure('a');
    });
    expect(view.result.current.error).toBeNull();
    expect(view.result.current.sessions.has('a')).toBe(true);
  },
);
