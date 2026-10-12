import type * as TauriCore from '@tauri-apps/api/core';
import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { buildDayTimeline } from './day-timeline';
import { dayTimelineFixture } from './day-timeline-fixtures';
import { useDayMutations, type DayPrivacyOperation } from './use-day-mutations';
import { useDayResults } from './use-day-results';

const native = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof TauriCore>()),
  invoke: native.invoke,
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: native.listen }));
let changed: (event: { payload: unknown }) => void;
function reply(command: string, id: string): unknown {
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
  return undefined;
}
beforeEach(() => {
  vi.resetAllMocks();
  native.listen.mockImplementation(
    async (_name: string, handler: (event: { payload: unknown }) => void) => {
      changed = handler;
      return () => undefined;
    },
  );
  native.invoke.mockImplementation(
    async (command: string, args?: { input: { recordingId: string } }) =>
      reply(command, args?.input.recordingId ?? ''),
  );
});
afterEach(cleanup);

test('processing invalidation clears published reads immediately and rejects a late OCR page before recovering', async () => {
  let finishOld: (value: unknown) => void = () => {
    throw new Error('Old page was not requested');
  };
  let pending = true;
  native.invoke.mockImplementation(
    async (command: string, args: { input: { recordingId: string } }) => {
      const id = args.input.recordingId;
      if (command === 'getOcrTimelineFrames' && id === 'a' && pending) {
        pending = false;
        return new Promise((resolve) => {
          finishOld = resolve;
        });
      }
      return reply(command, id);
    },
  );
  const model = buildDayTimeline(dayTimelineFixture());
  const view = renderHook(() => useDayResults(model));
  await waitFor(() => expect(view.result.current.results.has('b')).toBe(true));
  act(() => changed({ payload: {} }));
  expect(view.result.current.results.size).toBe(0);
  await act(async () => {
    finishOld(reply('getOcrTimelineFrames', 'a'));
  });
  expect(view.result.current.results.has('a')).toBe(false);
  await waitFor(() => expect(view.result.current.results.size).toBe(2));
  expect(view.result.current.error).toBe(false);
});

test('repeated OCR pagination fails visibly without publishing the broken recording, then Refresh retries it', async () => {
  const model = buildDayTimeline(dayTimelineFixture());
  let repeat = true;
  native.invoke.mockImplementation(
    async (command: string, args: { input: { recordingId: string } }) => {
      const id = args.input.recordingId;
      if (command === 'getOcrTimelineFrames' && id === 'a' && repeat)
        return {
          recordingId: id,
          frames: [],
          nextCursor: { id: 'same-page', timestampMs: 1 },
        };
      return reply(command, id);
    },
  );
  const view = renderHook(() => useDayResults(model));
  await waitFor(() => expect(view.result.current.error).toBe(true));
  expect(view.result.current.results.has('a')).toBe(false);
  expect(
    native.invoke.mock.calls.filter(
      ([cmd, args]) =>
        cmd === 'getOcrTimelineFrames' && args.input.recordingId === 'a',
    ),
  ).toHaveLength(2);
  repeat = false;
  act(() => view.result.current.reload());
  await waitFor(() => expect(view.result.current.results.size).toBe(2));
  expect(view.result.current.error).toBe(false);
});

test('a missing invalidation subscription clears results and reports an error; a late subscription is cleaned up', async () => {
  const model = buildDayTimeline(dayTimelineFixture());
  native.listen.mockRejectedValue(new Error('Synthetic subscription failed'));
  const view = renderHook(() => useDayResults(model));
  await waitFor(() => expect(view.result.current.error).toBe(true));
  expect(view.result.current.results.size).toBe(0);
  view.unmount();
  let finish: (cleanup: () => void) => void = () => {};
  native.listen.mockImplementation(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const next = renderHook(() => useDayResults(model));
  next.unmount();
  const unlisten = vi.fn();
  await act(async () => finish(unlisten));
  expect(unlisten).toHaveBeenCalledOnce();
});

test('unhide, mask deletion and resize retain original inputs and retry only failed writes', async () => {
  const refresh = vi.fn();
  let fail = true;
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'resizeLibraryRedaction' && fail)
      throw new Error('Synthetic resize conflict');
    return undefined;
  });
  const view = renderHook(() => useDayMutations('2026-10-11', refresh));
  const operations: DayPrivacyOperation[] = [
    {
      kind: 'ranges',
      recordingId: 'a',
      hide: false,
      ranges: [
        { startOffsetMs: 21000, endOffsetMs: 22000, trackIds: ['a-screen-0'] },
      ],
    },
    { kind: 'delete', recordingId: 'b', redactionId: 'mask-b' },
    {
      kind: 'resize',
      input: {
        recordingId: 'a',
        redactionId: 'mask-a',
        startOffsetMs: 21000,
        endOffsetMs: 22000,
        expectedStartOffsetMs: 20000,
        expectedEndOffsetMs: 22000,
      },
    },
  ];
  await act(async () => {
    await expect(view.result.current.run(operations)).rejects.toThrow();
  });
  expect(view.result.current.failedCount).toBe(1);
  expect(native.invoke).toHaveBeenCalledWith('unredactLibraryRedactions', {
    input: {
      recordingId: 'a',
      ranges: operations[0]?.kind === 'ranges' ? operations[0].ranges : [],
    },
  });
  expect(native.invoke).toHaveBeenCalledWith('deleteLibraryRedaction', {
    input: { recordingId: 'b', redactionId: 'mask-b' },
  });
  fail = false;
  act(() => view.result.current.retry());
  await waitFor(() => expect(view.result.current.failedCount).toBe(0));
  expect(
    native.invoke.mock.calls.filter(
      ([cmd]) => cmd === 'deleteLibraryRedaction',
    ),
  ).toHaveLength(1);
  expect(
    native.invoke.mock.calls.filter(
      ([cmd]) => cmd === 'resizeLibraryRedaction',
    ),
  ).toHaveLength(2);
  expect(refresh).toHaveBeenCalledTimes(2);
});

test('concurrent privacy writes are rejected and a retired day cannot publish errors or retry into the next day', async () => {
  const refresh = vi.fn();
  let failOld: (error: Error) => void = () => {
    throw new Error('No old write');
  };
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'createLibraryRedactions')
      return new Promise((_resolve, reject) => {
        failOld = reject;
      });
    return undefined;
  });
  const view = renderHook(({ day }) => useDayMutations(day, refresh), {
    initialProps: { day: '2026-10-11' },
  });
  const operation: DayPrivacyOperation = {
    kind: 'ranges',
    recordingId: 'a',
    hide: true,
    ranges: [
      { startOffsetMs: 21000, endOffsetMs: 22000, trackIds: ['a-screen-0'] },
    ],
  };
  let original: Promise<unknown> = Promise.resolve();
  act(() => {
    original = view.result.current.run([operation]).catch((error) => error);
  });
  expect(view.result.current.pending).toBe(true);
  await expect(view.result.current.run([operation])).rejects.toThrow(
    'already being saved',
  );
  view.rerender({ day: '2026-10-12' });
  await act(async () => {
    failOld(new Error('Synthetic write failed'));
    await original;
  });
  expect(view.result.current.pending).toBe(false);
  expect(view.result.current.failedCount).toBe(0);
  act(() => view.result.current.retry());
  expect(refresh).not.toHaveBeenCalled();
  expect(
    native.invoke.mock.calls.filter(
      ([cmd]) => cmd === 'createLibraryRedactions',
    ),
  ).toHaveLength(1);
});
