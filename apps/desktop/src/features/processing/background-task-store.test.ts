import type { BackgroundTaskPage } from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import { createBackgroundTaskStore } from './background-task-store';
import { task } from './testing/processing-browser-fixtures';

const client = vi.hoisted(() => ({ listBackgroundTasks: vi.fn() }));
vi.mock('./processing-client', () => client);
beforeEach(() => vi.resetAllMocks());
test('library invalidation fences late tasks and clears history pagination', async () => {
  let finish: ((page: BackgroundTaskPage) => void) | undefined;
  client.listBackgroundTasks
    .mockReturnValueOnce(
      new Promise<BackgroundTaskPage>((resolve) => {
        finish = resolve;
      }),
    )
    .mockResolvedValueOnce({ tasks: [], nextCursor: null });
  const store = createBackgroundTaskStore();
  store.setState({
    tasks: [task()],
    historyLoaded: true,
    nextCursor: { updatedAtMs: 1, id: 'synthetic-old' },
  });
  const old = store.getState().refresh();
  store.getState().invalidateLibrary();
  await store.getState().refresh();
  finish?.({
    tasks: [task()],
    nextCursor: { updatedAtMs: 1, id: 'synthetic-old' },
  });
  await old;
  expect(store.getState().tasks).toEqual([]);
  expect(store.getState().nextCursor).toBeNull();
  expect(store.getState().historyLoaded).toBe(false);
});
