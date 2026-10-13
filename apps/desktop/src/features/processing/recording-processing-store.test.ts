import type { RecordingProcessingGroupPage } from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import { createRecordingProcessingStore } from './recording-processing-store';
import { group, groupPage } from './testing/processing-browser-fixtures';

test('scoped reads include the whole day and keep loaded row order stable through progress updates', async () => {
  const a = group({ recordingId: 'a', updatedAtMs: 3000 }),
    b = group({ recordingId: 'b', updatedAtMs: 2000 });
  client.listRecordingProcessingGroups
    .mockResolvedValueOnce(groupPage([a, b]))
    .mockResolvedValueOnce(
      groupPage([{ ...b, updatedAtMs: 5000 }, a], {
        reconciledGroups: [a, { ...b, updatedAtMs: 5000 }],
      }),
    );
  const store = createRecordingProcessingStore(['a', 'b'], 'all');
  await store.getState().refresh();
  await store.getState().refresh();
  expect(client.listRecordingProcessingGroups).toHaveBeenLastCalledWith({
    recordingIds: ['a', 'b'],
    filter: 'all',
    cursor: null,
    knownRecordingIds: ['a', 'b'],
  });
  expect(store.getState().groups.map((group) => group.recordingId)).toEqual([
    'a',
    'b',
  ]);
});

const client = vi.hoisted(() => ({ listRecordingProcessingGroups: vi.fn() }));
vi.mock('./processing-client', () => client);
beforeEach(() => vi.resetAllMocks());

test('late failures cannot erase a new filter and equal timestamps have stable source identity order', async () => {
  let reject: ((error: unknown) => void) | undefined;
  client.listRecordingProcessingGroups
    .mockReturnValueOnce(
      new Promise((_, fail) => {
        reject = fail;
      }),
    )
    .mockResolvedValue(
      groupPage([group({ recordingId: 'a' }), group({ recordingId: 'b' })]),
    );
  const store = createRecordingProcessingStore();
  const first = store.getState().refresh();
  store.getState().setFilter('all');
  await vi.waitFor(() => expect(store.getState().status).toBe('ready'));
  reject?.({ code: 'ai_processing_storage_failed' });
  await first;
  expect(store.getState().groups.map((group) => group.recordingId)).toEqual([
    'b',
    'a',
  ]);
  expect(store.getState().error).toBeNull();
});

test('initial read failure is recoverable and pagination without a cursor never starts work', async () => {
  client.listRecordingProcessingGroups
    .mockRejectedValueOnce({ code: 'ai_processing_storage_failed' })
    .mockResolvedValue(groupPage([]));
  const store = createRecordingProcessingStore();
  await store.getState().loadMore();
  expect(client.listRecordingProcessingGroups).not.toHaveBeenCalled();
  await store.getState().refresh();
  expect(store.getState().status).toBe('error');
  expect(store.getState().error).toBeTruthy();
  await store.getState().refresh();
  expect(store.getState().status).toBe('ready');
  expect(store.getState().error).toBeNull();
});
test.each(['all', 'history', 'attention'] as const)(
  'loaded cards reconcile according to the %s filter',
  async (filter) => {
    const completed = group({ recordingId: 'completed', status: 'succeeded' }),
      active = group({ recordingId: 'active' }),
      failed = group({ recordingId: 'failed', status: 'failed' });
    client.listRecordingProcessingGroups
      .mockResolvedValueOnce(groupPage([completed, active, failed]))
      .mockResolvedValueOnce(
        groupPage([], { reconciledGroups: [completed, active, failed] }),
      );
    const store = createRecordingProcessingStore([], filter);
    await store.getState().refresh();
    await store.getState().refresh();
    expect(
      store
        .getState()
        .groups.map((group) => group.recordingId)
        .sort(),
    ).toEqual(
      filter === 'all'
        ? ['active', 'completed', 'failed']
        : filter === 'history'
          ? ['completed', 'failed']
          : ['failed'],
    );
    store.getState().setFilter(filter);
    expect(client.listRecordingProcessingGroups).toHaveBeenCalledTimes(2);
  },
);
test('events during a pending page coalesce into one refresh, and failed pagination retains loaded cards', async () => {
  let finish: ((value: RecordingProcessingGroupPage) => void) | undefined;
  client.listRecordingProcessingGroups
    .mockReturnValueOnce(
      new Promise<RecordingProcessingGroupPage>((resolve) => {
        finish = resolve;
      }),
    )
    .mockResolvedValueOnce(
      groupPage([group()], { nextCursor: { id: 'cursor', updatedAtMs: 1 } }),
    )
    .mockRejectedValueOnce({ code: 'ai_processing_storage_failed' });
  const store = createRecordingProcessingStore();
  const first = store.getState().refresh();
  await store.getState().refresh();
  await store.getState().loadMore();
  finish?.(groupPage([group()]));
  await first;
  await vi.waitFor(() => expect(store.getState().loading).toBe(false));
  expect(client.listRecordingProcessingGroups).toHaveBeenCalledTimes(2);
  await store.getState().loadMore();
  expect(store.getState().groups).toHaveLength(1);
  expect(store.getState().status).toBe('ready');
  expect(store.getState().error).toBeTruthy();
});

test('library invalidation discards loaded cards and fences a delayed recording page', async () => {
  let finish: ((page: RecordingProcessingGroupPage) => void) | undefined;
  client.listRecordingProcessingGroups
    .mockReturnValueOnce(
      new Promise<RecordingProcessingGroupPage>((resolve) => {
        finish = resolve;
      }),
    )
    .mockResolvedValueOnce(groupPage([]));
  const store = createRecordingProcessingStore();
  store.setState({
    groups: [group()],
    counts: { all: 1, active: 1, attention: 0, history: 0 },
  });
  const old = store.getState().refresh();
  store.getState().invalidateLibrary();
  await store.getState().refresh();
  finish?.(groupPage([group()]));
  await old;
  expect(store.getState().groups).toEqual([]);
  expect(store.getState().counts.all).toBe(0);
  expect(store.getState().nextCursor).toBeNull();
});

test('a late first-page response cannot replace the newly selected filter', async () => {
  let finish: ((page: RecordingProcessingGroupPage) => void) | undefined;
  client.listRecordingProcessingGroups
    .mockReturnValueOnce(
      new Promise<RecordingProcessingGroupPage>((resolve) => {
        finish = resolve;
      }),
    )
    .mockResolvedValueOnce(groupPage([]));
  const store = createRecordingProcessingStore();
  const first = store.getState().refresh();
  store.getState().setFilter('attention');
  await vi.waitFor(() => expect(store.getState().status).toBe('ready'));
  finish?.(groupPage([group()]));
  await first;
  expect(store.getState().filter).toBe('attention');
  expect(store.getState().groups).toEqual([]);
});

test('refresh reconciles loaded history and removes a deleted recording without discarding surviving older cards', async () => {
  const current = group();
  const deleted = group({ recordingId: 'deleted-recording', updatedAtMs: 500 });
  const older = group({ recordingId: 'older-recording', updatedAtMs: 100 });
  client.listRecordingProcessingGroups
    .mockResolvedValueOnce(
      groupPage([current], {
        nextCursor: { updatedAtMs: 3000, id: current.recordingId },
      }),
    )
    .mockResolvedValueOnce(groupPage([deleted, older]))
    .mockResolvedValueOnce(
      groupPage([current], {
        reconciledGroups: [current, older],
        nextCursor: { updatedAtMs: 3000, id: current.recordingId },
      }),
    );
  const store = createRecordingProcessingStore();
  await store.getState().refresh();
  await store.getState().loadMore();
  await store.getState().refresh();
  expect(store.getState().groups.map((group) => group.recordingId)).toEqual([
    current.recordingId,
    older.recordingId,
  ]);
  expect(store.getState().nextCursor).toBeNull();
  expect(client.listRecordingProcessingGroups).toHaveBeenLastCalledWith({
    filter: 'active',
    cursor: null,
    knownRecordingIds: [
      current.recordingId,
      deleted.recordingId,
      older.recordingId,
    ],
  });
});
