import type {
  LibraryRecordingDetail,
  LibraryWaveformTiles,
} from '@daydream/shared';
import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { getRecordingWaveformTiles } from '../../library-client';
import {
  studioStoryRecording,
  deferredStory,
} from '../../recording-studio-route/stories/studio-route-fixture';
import { useWaveformTileQueue } from './use-waveform-tile-queue';

// Native IPC and its invalidation subscription are the external boundaries.
const native = vi.hoisted(() => ({ listen: vi.fn() }));
vi.mock('@/features/ai/ai-client', () => ({
  listenToAiProcessingChanged: native.listen,
}));
afterEach(cleanup);
beforeEach(() => {
  native.listen.mockReset().mockResolvedValue(() => undefined);
});
const detail = { ...studioStoryRecording, availability: 'available' as const };
const tiles: LibraryWaveformTiles = {
  recordingId: detail.id,
  binMs: 10,
  error: null,
  pending: false,
  tracks: [],
};
const options = {
  detail,
  requestRange: { startTimelineMs: 0, endTimelineMs: 10_000 },
  timelineWidthPx: 1000,
  totalDurationMs: 10_000,
};
const recordingProps: { recording: LibraryRecordingDetail } = {
  recording: detail,
};

test('pending waveform publications retry invalid responses and cache the recovered result', async () => {
  const read = vi
    .fn<typeof getRecordingWaveformTiles>()
    .mockResolvedValueOnce({ ...tiles, pending: true })
    .mockResolvedValueOnce({ ...tiles, recordingId: 'another-recording' })
    .mockResolvedValue(tiles);
  const view = renderHook(() =>
    useWaveformTileQueue({ ...options, readTiles: read }),
  );
  await waitFor(() =>
    expect(view.result.current.waveformTiles?.pending).toBe(true),
  );
  expect(view.result.current.waveformStatus).toBe('loading');
  const listener = native.listen.mock.calls[0]![0];
  act(() => {
    listener({ recordingId: 'unrelated', sourceKind: 'waveform' });
    listener({ recordingId: detail.id, sourceKind: 'waveform' });
    listener({ recordingId: detail.id, sourceKind: 'waveform' });
  });
  await waitFor(() => expect(view.result.current.waveformStatus).toBe('error'));
  expect(read).toHaveBeenCalledTimes(2);
  expect(view.result.current.waveformTiles).toBeNull();
  act(() => listener({ recordingId: detail.id, sourceKind: 'waveform' }));
  await waitFor(() => expect(view.result.current.waveformStatus).toBe('ready'));
  expect(view.result.current.waveformTiles).toEqual(tiles);
  view.unmount();
  const warm = renderHook(() =>
    useWaveformTileQueue({ ...options, readTiles: read }),
  );
  expect(warm.result.current.waveformTiles).toEqual(tiles);
  await act(async () => undefined);
  expect(read).toHaveBeenCalledTimes(3);
});

test('the final publication during an unresolved read refreshes pending tiles without another event', async () => {
  const response = deferredStory<LibraryWaveformTiles>();
  const read = vi
    .fn<typeof getRecordingWaveformTiles>()
    .mockReturnValueOnce(response.promise)
    .mockResolvedValue(tiles);
  const view = renderHook(() =>
    useWaveformTileQueue({ ...options, readTiles: read }),
  );
  await waitFor(() => expect(native.listen).toHaveBeenCalledOnce());
  const listener = native.listen.mock.calls[0]![0];
  act(() => {
    listener({ recordingId: detail.id, sourceKind: 'waveform' });
    listener({ recordingId: detail.id, sourceKind: 'waveform' });
  });
  expect(read).toHaveBeenCalledOnce();
  await act(async () => response.resolve({ ...tiles, pending: true }));
  await waitFor(() => expect(view.result.current.waveformStatus).toBe('ready'));
  expect(view.result.current.waveformTiles).toEqual(tiles);
  expect(read).toHaveBeenCalledTimes(2);
  act(() => listener({ recordingId: detail.id, sourceKind: 'waveform' }));
  expect(read).toHaveBeenCalledTimes(2);
});

test('publication during a failed tile read also recovers without a later event', async () => {
  const response = deferredStory<LibraryWaveformTiles>();
  const read = vi
    .fn<typeof getRecordingWaveformTiles>()
    .mockReturnValueOnce(response.promise)
    .mockResolvedValue(tiles);
  const view = renderHook(() =>
    useWaveformTileQueue({ ...options, readTiles: read }),
  );
  await waitFor(() => expect(native.listen).toHaveBeenCalledOnce());
  act(() =>
    native.listen.mock.calls[0]![0]({
      recordingId: detail.id,
      sourceKind: 'waveform',
    }),
  );
  await act(async () =>
    response.resolve({ ...tiles, error: 'synthetic-cache-failure' }),
  );
  await waitFor(() => expect(view.result.current.waveformStatus).toBe('ready'));
  expect(view.result.current.waveformTiles).toEqual(tiles);
  expect(read).toHaveBeenCalledTimes(2);
});

test('a failed publication subscription still displays an already completed native read', async () => {
  native.listen.mockRejectedValue(
    new Error('Synthetic subscription unavailable'),
  );
  const read = vi
    .fn<typeof getRecordingWaveformTiles>()
    .mockResolvedValue(tiles);
  const view = renderHook(() =>
    useWaveformTileQueue({ ...options, readTiles: read }),
  );
  await waitFor(() => expect(view.result.current.waveformStatus).toBe('ready'));
  expect(view.result.current.waveformTiles).toEqual(tiles);
  expect(read).toHaveBeenCalledOnce();
});

test('locking a recording withholds cached waveforms without issuing another read', async () => {
  const read = vi
    .fn<typeof getRecordingWaveformTiles>()
    .mockResolvedValue(tiles);
  const view = renderHook(
    ({ recording }: { recording: LibraryRecordingDetail }) =>
      useWaveformTileQueue({ ...options, detail: recording, readTiles: read }),
    { initialProps: recordingProps },
  );
  await waitFor(() => expect(view.result.current.waveformStatus).toBe('ready'));
  view.rerender({ recording: { ...options.detail, availability: 'locked' } });
  expect(view.result.current.waveformTiles).toBeNull();
  expect(view.result.current.waveformStatus).toBe('idle');
  expect(read).toHaveBeenCalledOnce();
});

test('a privacy revision withholds the previous sample while replacement tiles load', async () => {
  const response = deferredStory<LibraryWaveformTiles>();
  const read = vi
    .fn<typeof getRecordingWaveformTiles>()
    .mockResolvedValueOnce(tiles)
    .mockReturnValueOnce(response.promise);
  const view = renderHook(
    ({ recording }: { recording: LibraryRecordingDetail }) =>
      useWaveformTileQueue({ ...options, detail: recording, readTiles: read }),
    { initialProps: recordingProps },
  );
  await waitFor(() => expect(view.result.current.waveformStatus).toBe('ready'));
  view.rerender({
    recording: {
      ...options.detail,
      redactions: [
        {
          id: '00000000-0000-7000-8000-000000000001',
          recordingId: detail.id,
          startOffsetMs: 0,
          endOffsetMs: 1000,
          trackIds: ['microphone-2'],
          reason: null,
          createdAtMs: 1,
        },
      ],
    },
  });
  expect(view.result.current.waveformTiles).toBeNull();
  expect(view.result.current.waveformStatus).toBe('loading');
  await act(async () => response.resolve(tiles));
  await waitFor(() => expect(view.result.current.waveformStatus).toBe('ready'));
  expect(read).toHaveBeenCalledTimes(2);
});

test('panning through a long recording bounds the cache and rereads an evicted viewport', async () => {
  const duration = 600_000;
  const recording = {
    ...detail,
    durationMs: duration,
    segments: detail.segments.map((segment) => ({
      ...segment,
      durationMs: duration,
    })),
  };
  const read = vi
    .fn<typeof getRecordingWaveformTiles>()
    .mockResolvedValue({ ...tiles, binMs: 500 });
  const initial = { startTimelineMs: 0, endTimelineMs: 10_000 };
  const view = renderHook(
    ({ range }) =>
      useWaveformTileQueue({
        ...options,
        detail: recording,
        totalDurationMs: duration,
        requestRange: range,
        readTiles: read,
      }),
    { initialProps: { range: initial } },
  );
  await waitFor(() => expect(view.result.current.waveformStatus).toBe('ready'));
  for (let index = 1; index < 33; index += 1) {
    view.rerender({
      range: {
        startTimelineMs: index * 15_000,
        endTimelineMs: index * 15_000 + 10_000,
      },
    });
    await waitFor(() =>
      expect(view.result.current.waveformStatus).toBe('ready'),
    );
  }
  expect(read).toHaveBeenCalledTimes(33);
  view.rerender({ range: initial });
  await waitFor(() => expect(view.result.current.waveformStatus).toBe('ready'));
  expect(read).toHaveBeenCalledTimes(34);
});

test('retired waveform owners release late subscriptions and reject late foreign data', async () => {
  const response = deferredStory<LibraryWaveformTiles>();
  const subscription = deferredStory<() => void>();
  const unlisten = vi.fn();
  native.listen.mockReturnValue(subscription.promise);
  const read = vi
    .fn<typeof getRecordingWaveformTiles>()
    .mockReturnValue(response.promise);
  const view = renderHook(() =>
    useWaveformTileQueue({ ...options, readTiles: read }),
  );
  view.unmount();
  await act(async () => {
    response.resolve({ ...tiles, recordingId: 'foreign' });
    subscription.resolve(unlisten);
  });
  expect(unlisten).toHaveBeenCalledOnce();
  read.mockResolvedValue(tiles);
  const replacement = renderHook(() =>
    useWaveformTileQueue({ ...options, readTiles: read }),
  );
  await waitFor(() =>
    expect(replacement.result.current.waveformTiles).toEqual(tiles),
  );
  expect(read).toHaveBeenCalledTimes(2);
});
