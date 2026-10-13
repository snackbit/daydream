import type * as TauriCore from '@tauri-apps/api/core';
import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { useStudioResultWindowStore as windows } from '../components/studio/studio-result-window-store';
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
  windows.setState(windows.getInitialState(), true);
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

test('recording processing refresh retains other published reads and rejects a late OCR page before recovering', async () => {
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
  act(() => changed({ payload: { recordingId: 'a' } }));
  expect(view.result.current.results.has('b')).toBe(true);
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

test('coarse day reads request source overview ranges, skip OCR detail, and ignore waveform publications', async () => {
  const model = buildDayTimeline(dayTimelineFixture());
  windows.getState().setWindow(model.view.id, 0, 18_000, 0.0001);
  windows.getState().setReview(model.view.id, 0);
  const view = renderHook(() => useDayResults(model));
  await waitFor(() => expect(view.result.current.results.size).toBe(2));
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === 'getOcrTimelineFrames',
    ),
  ).toBe(false);
  expect(native.invoke).toHaveBeenCalledWith('getRecordingAiProcessingState', {
    input: {
      recordingId: 'a',
      startOffsetMs: 21_000,
      endOffsetMs: 26_000,
      reviewStartOffsetMs: 21_000,
      reviewEndOffsetMs: 26_000,
      bucketMs: 480_000,
    },
  });
  const before = native.invoke.mock.calls.length;
  const original = view.result.current.results.get('a');
  act(() => changed({ payload: { recordingId: 'a', sourceKind: 'waveform' } }));
  act(() => changed({ payload: { recordingId: 'unrelated' } }));
  expect(native.invoke.mock.calls).toHaveLength(before);
  act(() =>
    windows.getState().setWindow(model.view.id, 60_000, 90_000, 0.0002),
  );
  await waitFor(() =>
    expect(native.invoke.mock.calls.length).toBeGreaterThan(before),
  );
  expect(view.result.current.results.size).toBe(2);
  const loaded = native.invoke.mock.calls.length;
  act(() => windows.getState().setWindow(model.view.id, 0, 18_000, 0.0001));
  await waitFor(() =>
    expect(view.result.current.results.get('a')).toBe(original),
  );
  expect(native.invoke.mock.calls).toHaveLength(loaded);
  act(() => changed({ payload: { recordingId: 'a' } }));
  await waitFor(() =>
    expect(native.invoke.mock.calls.length).toBeGreaterThan(loaded),
  );
  await waitFor(() =>
    expect(view.result.current.results.get('a')).not.toBe(original),
  );
});

test.each([
  ['same source', ['a-screen-0'], 0],
  ['all sources', [], 0],
  ['other source', ['a-screen-1'], 1],
] as const)(
  'new source redactions on %s apply to cached screen details before replacement IPC completes',
  async (_scope, trackIds, remaining) => {
    const fixture = dayTimelineFixture();
    native.invoke.mockImplementation(
      async (command: string, args: { input: { recordingId: string } }) => {
        const id = args.input.recordingId;
        if (command === 'getOcrTimelineFrames' && id === 'a')
          return {
            recordingId: id,
            nextCursor: null,
            frames: [
              {
                id: 'frame',
                recordingId: id,
                documentId: 'doc',
                segmentId: 'a-segment',
                sourceTrackId: 'a-screen-0',
                sourceFrameRef: 'synthetic',
                frameTimestampMs: 21_000,
                sampleEndMs: 22_000,
                status: 'succeeded',
                errorCode: null,
                ocrOutputCount: 1,
                textEmpty: false,
              },
            ],
          };
        return reply(command, id);
      },
    );
    const view = renderHook(({ model }) => useDayResults(model), {
      initialProps: { model: buildDayTimeline(fixture) },
    });
    await waitFor(() =>
      expect(view.result.current.results.get('a')?.frames).toHaveLength(1),
    );
    native.invoke.mockImplementation(() => new Promise(() => undefined));
    const protectedFixture = structuredClone(fixture);
    protectedFixture.recordings[0]!.redactions = [
      {
        id: 'cut',
        recordingId: 'a',
        createdAtMs: 1,
        startOffsetMs: 21_000,
        endOffsetMs: 22_000,
        trackIds: [...trackIds],
        reason: null,
      },
    ];
    view.rerender({ model: buildDayTimeline(protectedFixture) });
    expect(view.result.current.results.get('a')?.frames).toHaveLength(
      remaining,
    );
    expect(view.result.current.results.has('b')).toBe(true);
  },
);

test('redacting earlier comparison context withholds its cached observation before a new read completes', async () => {
  const fixture = dayTimelineFixture();
  native.invoke.mockImplementation(
    async (command: string, args: { input: { recordingId: string } }) => {
      const id = args.input.recordingId;
      if (command === 'getRecordingVisualObservations' && id === 'a')
        return {
          recordingId: id,
          ocrRegions: [],
          vlmObservations: [
            {
              id: 'observation',
              jobId: 'job',
              chunkId: 'chunk',
              recordingId: id,
              frameTimestampMs: 21_000,
              contextStartMs: 20_000,
              confirmedEndMs: 22_000,
              sourceFrameRef: 'a-segment:stream:a-screen-0:21000',
              region: { x: 0, y: 0, width: 640, height: 360 },
              provider: 'openai-compatible',
              model: 'synthetic',
              confidence: null,
              promptVersion: 'changes-v1',
              observationKind: 'screen-observation',
              text: 'Synthetic screen change',
            },
          ],
        };
      return reply(command, id);
    },
  );
  const view = renderHook(({ model }) => useDayResults(model), {
    initialProps: { model: buildDayTimeline(fixture) },
  });
  await waitFor(() =>
    expect(view.result.current.results.get('a')?.frames).toHaveLength(1),
  );
  native.invoke.mockImplementation(() => new Promise(() => undefined));
  const protectedFixture = structuredClone(fixture);
  protectedFixture.recordings[0]!.redactions = [
    {
      id: 'cut-context',
      recordingId: 'a',
      createdAtMs: 1,
      startOffsetMs: 20_000,
      endOffsetMs: 20_500,
      trackIds: ['a-screen-0'],
      reason: null,
    },
  ];
  view.rerender({ model: buildDayTimeline(protectedFixture) });
  expect(view.result.current.results.get('a')?.frames).toEqual([]);
});

test('a missing invalidation subscription preserves readable results, reports the degraded state and reconciles on focus; a late subscription is cleaned up', async () => {
  const model = buildDayTimeline(dayTimelineFixture());
  native.listen.mockRejectedValue(new Error('Synthetic subscription failed'));
  const view = renderHook(() => useDayResults(model));
  await waitFor(() => expect(view.result.current.error).toBe(true));
  await waitFor(() =>
    expect(view.result.current.results.size).toBeGreaterThan(0),
  );
  const previousReads = native.invoke.mock.calls.length;
  window.dispatchEvent(new Event('focus'));
  await waitFor(() =>
    expect(native.invoke.mock.calls.length).toBeGreaterThan(previousReads),
  );
  expect(view.result.current.error).toBe(true);
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
