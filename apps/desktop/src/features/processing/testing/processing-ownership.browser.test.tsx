import { useCallback, useState } from 'react';
import {
  createRootRoute,
  createRoute,
  createRouter,
  createMemoryHistory,
  RouterProvider,
} from '@tanstack/react-router';
import type {
  BackgroundTasksChanged,
  RecordingProcessingLaneDetail,
  RecordingProcessingUnit,
} from '@daydream/shared';
import {
  act,
  cleanup,
  render,
  renderHook,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import { useSettingsStore } from '@/features/settings/settings-store';
import { RecordingLaneDialog } from '../components/recording-lane-dialog';
import type * as ProcessingClient from '../processing-client';
import {
  createRecordingProcessingStore,
  useRecordingProcessingStore,
} from '../recording-processing-store';
import { useRecordingProcessingActions } from '../use-recording-processing-actions';
import { useRecordingProcessingSync } from '../use-recording-processing-sync';
import { groupPage, lane } from './processing-browser-fixtures';

const client = vi.hoisted(() => ({
  listRecordingProcessingGroups: vi.fn(),
  getRecordingProcessingLane: vi.fn(),
  controlRecordingProcessingGroups: vi.fn(),
  retryProcessingUnits: vi.fn(),
  listenToBackgroundTasksChanged: vi.fn(),
}));
vi.mock('../processing-client', async (importOriginal) => ({
  ...(await importOriginal<typeof ProcessingClient>()),
  ...client,
}));
let change: ((event: BackgroundTasksChanged) => void) | undefined;
beforeEach(() => {
  vi.resetAllMocks();
  change = undefined;
  useSettingsStore.setState({
    loadStatus: 'ready',
    draft: null,
    settingsState: null,
  });
  useRecordingProcessingStore.setState(
    useRecordingProcessingStore.getInitialState(),
    true,
  );
  client.listRecordingProcessingGroups.mockResolvedValue(groupPage([]));
  client.listenToBackgroundTasksChanged.mockImplementation(async (handler) => {
    change = handler;
    return () => undefined;
  });
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

test('a repeated native cursor cannot loop forever or discard already expanded source sections', async () => {
  const cursor = { chunkId: 'chunk', unitId: 'frame' };
  let corrupt = false;
  client.getRecordingProcessingLane.mockImplementation(async (input) =>
    corrupt
      ? detail({ units: [], nextCursor: cursor })
      : input.cursor
        ? detail({
            units: [
              {
                ...unit,
                id: 'second',
                startOffsetMs: 2000,
                status: 'succeeded',
                errorCode: null,
              },
            ],
          })
        : detail({
            units: [{ ...unit, status: 'succeeded', errorCode: null }],
            nextCursor: cursor,
          }),
  );
  mountDialog();
  await page.getByRole('button', { name: 'Load more sections' }).click();
  await screen.findByText('OCR · 2s');
  corrupt = true;
  change?.({ recordingId: 'recording', updatedAtMs: 3 });
  await screen.findByRole('alert', { name: 'Processing sections error' });
  expect(screen.getByText('OCR · 2s')).toBeTruthy();
  expect(client.getRecordingProcessingLane.mock.calls.length).toBeLessThan(10);
});

test('multiple action failures without a provider code remain visible and refresh their recording', async () => {
  client.controlRecordingProcessingGroups.mockResolvedValue({
    results: [
      { jobId: 'one', succeeded: false, errorCode: null },
      { jobId: 'two', succeeded: false, errorCode: null },
    ],
  });
  const view = renderHook(() => useRecordingProcessingActions('recording'));
  await act(async () => {
    await view.result.current.perform({ action: 'pause' });
  });
  expect(view.result.current.error).toContain(
    '2 processing jobs need attention',
  );
  expect(client.listRecordingProcessingGroups).toHaveBeenCalledOnce();
});

test('read failures reconcile on the fallback clock, duplicate events coalesce, and idle or disposed synchronizers stop reading', async () => {
  vi.useFakeTimers();
  client.listRecordingProcessingGroups
    .mockRejectedValueOnce({ code: 'ai_processing_storage_failed' })
    .mockResolvedValue(groupPage([]));
  const store = createRecordingProcessingStore();
  const view = renderHook(() => useRecordingProcessingSync(store));
  await act(async () => {});
  expect(store.getState().status).toBe('error');
  await act(async () => {
    await vi.advanceTimersByTimeAsync(5500);
  });
  expect(store.getState().status).toBe('ready');
  const handler = change;
  await act(async () => {
    handler?.({ recordingId: 'recording', updatedAtMs: 1 });
    handler?.({ recordingId: 'recording', updatedAtMs: 2 });
    await vi.advanceTimersByTimeAsync(500);
  });
  expect(client.listRecordingProcessingGroups).toHaveBeenCalledTimes(3);
  await act(async () => {
    await vi.advanceTimersByTimeAsync(5000);
  });
  expect(client.listRecordingProcessingGroups).toHaveBeenCalledTimes(3);
  view.unmount();
  handler?.({ recordingId: 'recording', updatedAtMs: 3 });
  await vi.advanceTimersByTimeAsync(500);
  expect(client.listRecordingProcessingGroups).toHaveBeenCalledTimes(3);
});

test('background reconciliation preserves expanded section pages and a source without a known track still links to its recording time', async () => {
  const cursor = { chunkId: 'chunk', unitId: 'frame' };
  client.getRecordingProcessingLane.mockImplementation(async (input) =>
    input.cursor
      ? detail({
          units: [
            {
              ...unit,
              id: 'second',
              startOffsetMs: 3000,
              sourceTrackId: null,
              status: 'succeeded',
              errorCode: null,
            },
          ],
        })
      : detail({
          units: [{ ...unit, status: 'succeeded', errorCode: null }],
          nextCursor: cursor,
        }),
  );
  mountDialog();
  await page.getByRole('button', { name: 'Load more sections' }).click();
  await screen.findByText('OCR · 3s');
  change?.({ recordingId: 'recording', updatedAtMs: 2 });
  await waitFor(() =>
    expect(
      client.getRecordingProcessingLane.mock.calls.length,
    ).toBeGreaterThanOrEqual(4),
  );
  expect(screen.getByText('OCR · 3s')).toBeTruthy();
  expect(
    screen
      .getAllByRole('link', { name: 'Open source position' })
      .some((link) => link.getAttribute('href')?.includes('seek=3000')),
  ).toBe(true);
  await page
    .getByRole('button', { name: 'Close', exact: true })
    .first()
    .click();
  expect(screen.queryByRole('dialog')).toBeNull();
});

test.each(['resolve', 'reject'] as const)(
  'a pagination response cannot change a closed dialog when it eventually %ss',
  async (outcome) => {
    let finish: ((value: RecordingProcessingLaneDetail) => void) | undefined,
      fail: ((error: unknown) => void) | undefined;
    client.getRecordingProcessingLane
      .mockResolvedValueOnce(
        detail({ nextCursor: { chunkId: 'chunk', unitId: 'frame' } }),
      )
      .mockReturnValueOnce(
        new Promise<RecordingProcessingLaneDetail>((resolve, reject) => {
          finish = resolve;
          fail = reject;
        }),
      );
    const view = mountDialog();
    await page.getByRole('button', { name: 'Load more sections' }).click();
    await page
      .getByRole('button', { name: 'Close', exact: true })
      .first()
      .click();
    await act(async () => {
      if (outcome === 'resolve') finish?.(detail());
      else fail?.({ code: 'ai_processing_storage_failed' });
    });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
    view.unmount();
  },
);
const unit: RecordingProcessingUnit = {
  id: 'frame',
  chunkId: 'chunk',
  jobId: 'job',
  sourceKind: 'ocr',
  sourceTrackId: 'screen',
  sourceRole: null,
  startOffsetMs: 1000,
  endOffsetMs: null,
  status: 'failed',
  progress: 1,
  errorCode: 'ai_ocr_vision_timeout',
  attemptCount: 1,
};
function detail(
  patch: Partial<RecordingProcessingLaneDetail> = {},
): RecordingProcessingLaneDetail {
  return { lane: lane('ocr'), units: [unit], nextCursor: null, ...patch };
}
function mountDialog() {
  const root = createRootRoute();
  const index = createRoute({
    getParentRoute: () => root,
    path: '/',
    component: Dialog,
  });
  const source = createRoute({
    getParentRoute: () => root,
    path: '/library/$recordingId',
  });
  const settings = createRoute({
    getParentRoute: () => root,
    path: '/settings',
  });
  return render(
    <RouterProvider
      router={createRouter({
        routeTree: root.addChildren([index, source, settings]),
        history: createMemoryHistory({ initialEntries: ['/'] }),
      })}
    />,
  );
}
function Dialog() {
  const [open, setOpen] = useState(true);
  const close = useCallback(() => setOpen(false), []);
  return (
    <RecordingLaneDialog
      selection={open ? { recordingId: 'recording', kind: 'ocr' } : null}
      onClose={close}
    />
  );
}

test('overlapping processing actions are admitted once and partial failures, empty results and transport errors remain recoverable', async () => {
  let finish: ((value: unknown) => void) | undefined;
  client.controlRecordingProcessingGroups
    .mockReturnValueOnce(
      new Promise((resolve) => {
        finish = resolve;
      }),
    )
    .mockResolvedValueOnce({ results: [] })
    .mockRejectedValueOnce({ code: 'ai_processing_storage_failed' })
    .mockResolvedValue({
      results: [{ jobId: 'job', succeeded: true, errorCode: null }],
    });
  const view = renderHook(() => useRecordingProcessingActions('recording'));
  let first: Promise<void> | undefined;
  await act(async () => {
    first = view.result.current.perform({ action: 'pause' });
    await view.result.current.perform({ action: 'pause' });
  });
  expect(client.controlRecordingProcessingGroups).toHaveBeenCalledTimes(1);
  expect(view.result.current.pending).toBe(true);
  await act(async () => {
    finish?.({
      results: [
        {
          jobId: 'job',
          succeeded: false,
          errorCode: 'ai_processing_job_worker_failed',
        },
      ],
    });
    await first;
  });
  expect(view.result.current.error).toContain('worker stopped');
  await act(async () => {
    await view.result.current.perform({ action: 'resume' });
  });
  expect(view.result.current.error).toContain('no longer available');
  await act(async () => {
    await view.result.current.perform({ action: 'resume' });
  });
  expect(view.result.current.error).toBeTruthy();
  await act(async () => {
    await view.result.current.perform({ action: 'resume' });
  });
  expect(view.result.current.error).toBeNull();
  expect(view.result.current.pending).toBe(false);
});

test('inactive synchronizers do not fetch; unavailable events still reconcile idle work, and late subscriptions clean up', async () => {
  const store = createRecordingProcessingStore();
  const idle = renderHook(() => useRecordingProcessingSync(store, false));
  expect(client.listRecordingProcessingGroups).not.toHaveBeenCalled();
  idle.unmount();
  client.listenToBackgroundTasksChanged.mockRejectedValueOnce({
    code: 'subscription_unavailable',
  });
  const view = renderHook(() => useRecordingProcessingSync(store));
  await waitFor(() =>
    expect(client.listRecordingProcessingGroups).toHaveBeenCalledTimes(2),
  );
  await waitFor(
    () => expect(client.listRecordingProcessingGroups).toHaveBeenCalledTimes(3),
    { timeout: 6500 },
  );
  view.unmount();
  let finish: ((cleanup: () => void) => void) | undefined;
  client.listenToBackgroundTasksChanged.mockReturnValueOnce(
    new Promise<() => void>((resolve) => {
      finish = resolve;
    }),
  );
  const next = renderHook(() =>
    useRecordingProcessingSync(createRecordingProcessingStore()),
  );
  next.unmount();
  const unlisten = vi.fn();
  await act(async () => finish?.(unlisten));
  expect(unlisten).toHaveBeenCalledOnce();
}, 9000);

test('a newer section read owns the dialog; a late older result and events for other recordings cannot replace it', async () => {
  let finish: ((value: RecordingProcessingLaneDetail) => void) | undefined;
  let changedSource = false;
  const old = new Promise<RecordingProcessingLaneDetail>((resolve) => {
    finish = resolve;
  });
  client.getRecordingProcessingLane.mockImplementation(() =>
    changedSource
      ? Promise.resolve(
          detail({
            units: [
              {
                ...unit,
                id: 'new',
                startOffsetMs: 2000,
                status: 'succeeded',
                errorCode: null,
              },
            ],
          }),
        )
      : old,
  );
  mountDialog();
  await screen.findByText('Loading sections…');
  const previousReads = client.getRecordingProcessingLane.mock.calls.length;
  change?.({ recordingId: 'other', updatedAtMs: 1 });
  expect(client.getRecordingProcessingLane).toHaveBeenCalledTimes(
    previousReads,
  );
  changedSource = true;
  change?.({ recordingId: 'recording', updatedAtMs: 2 });
  await screen.findByText('OCR · 2s');
  await act(async () => finish?.(detail()));
  expect(screen.queryByText('OCR · 1s')).toBeNull();
  expect(
    screen
      .getByRole('link', { name: 'Open source position' })
      .getAttribute('href'),
  ).toContain('seek=2000');
});

test('failed pagination preserves source sections and scoped retry re-reads the current native attempt', async () => {
  const cursor = { chunkId: 'chunk', unitId: 'frame' };
  client.getRecordingProcessingLane
    .mockResolvedValueOnce(detail({ nextCursor: cursor }))
    .mockRejectedValueOnce({ code: 'ai_processing_storage_failed' })
    .mockResolvedValueOnce(
      detail({
        units: [
          {
            ...unit,
            id: 'second',
            startOffsetMs: null,
            sourceTrackId: null,
            errorCode: null,
            status: 'succeeded',
          },
        ],
      }),
    )
    .mockResolvedValue(
      detail({ units: [{ ...unit, status: 'queued', errorCode: null }] }),
    );
  client.retryProcessingUnits.mockResolvedValue({
    id: 'job',
    status: 'queued',
  });
  mountDialog();
  await screen.findByRole('button', { name: 'Load more sections' });
  await page.getByRole('button', { name: 'Load more sections' }).click();
  await screen.findByRole('alert', { name: 'Processing sections error' });
  expect(screen.getByText('OCR · 1s')).toBeTruthy();
  await page.getByRole('button', { name: 'Load more sections' }).click();
  await screen.findByText('OCR · Whole source');
  expect(
    screen.queryByRole('alert', { name: 'Processing sections error' }),
  ).toBeNull();
  expect(
    screen.getAllByRole('link', { name: 'Open source position' }),
  ).toHaveLength(1);
  await page
    .getByRole('button', { name: 'Retry section', exact: true })
    .click();
  await waitFor(() =>
    expect(client.retryProcessingUnits).toHaveBeenCalledWith({
      jobId: 'job',
      chunkId: 'chunk',
      unitId: 'frame',
      useCurrentSettings: false,
    }),
  );
  await waitFor(() =>
    expect(screen.queryByRole('button', { name: 'Retry section' })).toBeNull(),
  );
});

test('closing a dialog with pending reads prevents stale failures and releases a late event subscription', async () => {
  let reject: ((error: unknown) => void) | undefined,
    finish: ((cleanup: () => void) => void) | undefined;
  client.getRecordingProcessingLane.mockReturnValue(
    new Promise((_, fail) => {
      reject = fail;
    }),
  );
  client.listenToBackgroundTasksChanged.mockImplementation((handler) => {
    change = handler;
    return new Promise<() => void>((resolve) => {
      finish = resolve;
    });
  });
  const view = mountDialog();
  await screen.findByText('Loading sections…');
  view.unmount();
  const unlisten = vi.fn();
  await act(async () => {
    change?.({ recordingId: 'recording', updatedAtMs: 4 });
    reject?.({ code: 'ai_processing_storage_failed' });
    finish?.(unlisten);
  });
  expect(screen.queryByRole('alert')).toBeNull();
  expect(unlisten).toHaveBeenCalledOnce();
});
