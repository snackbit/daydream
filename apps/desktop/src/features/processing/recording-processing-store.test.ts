import type { RecordingProcessingGroupPage } from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import { createRecordingProcessingStore } from './recording-processing-store';
import { group, groupPage } from './testing/processing-browser-fixtures';

const client = vi.hoisted(() => ({ listRecordingProcessingGroups: vi.fn() }));
vi.mock('./processing-client', () => client);
beforeEach(() => vi.resetAllMocks());

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
    filter: 'all',
    cursor: null,
    knownRecordingIds: [
      current.recordingId,
      deleted.recordingId,
      older.recordingId,
    ],
  });
});
