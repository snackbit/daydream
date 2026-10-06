import type { BackgroundTask, BackgroundTaskCursor } from '@daydream/shared';
import { create } from 'zustand';
import { listBackgroundTasks } from './processing-client';
import { processingFailureSummary } from './processing-errors';

export interface BackgroundTaskState {
  tasks: BackgroundTask[];
  nextCursor: BackgroundTaskCursor | null;
  status: 'idle' | 'loading' | 'ready' | 'error';
  error: string | null;
  loading: boolean;
  refreshQueued: boolean;
  historyLoaded: boolean;
  refresh: () => Promise<void>;
  loadMore: () => Promise<void>;
}

export function createBackgroundTaskStore() {
  return create<BackgroundTaskState>((set, get) => ({
    tasks: [],
    nextCursor: null,
    status: 'idle',
    error: null,
    loading: false,
    refreshQueued: false,
    historyLoaded: false,
    refresh: async () => {
      if (get().loading) {
        set({ refreshQueued: true });
        return;
      }
      set({
        loading: true,
        error: null,
        status: get().status === 'ready' ? 'ready' : 'loading',
      });
      try {
        do {
          set({ refreshQueued: false });
          const page = await listBackgroundTasks();
          const merged = new Map(
            get().historyLoaded
              ? get().tasks.map((task) => [task.id, task])
              : [],
          );
          for (const task of page.tasks) merged.set(task.id, task);
          set({
            tasks: [...merged.values()].sort(
              (a, b) =>
                b.updatedAtMs - a.updatedAtMs || b.id.localeCompare(a.id),
            ),
            nextCursor: get().historyLoaded
              ? get().nextCursor
              : page.nextCursor,
            status: 'ready',
            error: null,
          });
        } while (get().refreshQueued);
      } catch (error) {
        set({ status: 'error', error: processingFailureSummary(error) });
      } finally {
        set({ loading: false });
      }
    },
    loadMore: async () => {
      const cursor = get().nextCursor;
      if (!cursor || get().loading) return;
      set({ loading: true, error: null });
      try {
        const page = await listBackgroundTasks({ cursor });
        const tasks = new Map(get().tasks.map((task) => [task.id, task]));
        for (const task of page.tasks) tasks.set(task.id, task);
        set({
          tasks: [...tasks.values()],
          nextCursor: page.nextCursor,
          historyLoaded: true,
        });
      } catch (error) {
        set({ error: processingFailureSummary(error) });
      } finally {
        set({ loading: false });
        if (get().refreshQueued) void get().refresh();
      }
    },
  }));
}

export const useBackgroundTaskStore = createBackgroundTaskStore();
