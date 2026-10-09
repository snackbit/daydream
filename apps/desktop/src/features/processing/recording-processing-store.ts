import type {
  BackgroundTaskCursor,
  RecordingProcessingFilter,
  RecordingProcessingGroup,
  RecordingProcessingGroupPage,
} from '@daydream/shared';
import { create } from 'zustand';
import { listRecordingProcessingGroups } from './processing-client';
import { processingFailureSummary } from './processing-errors';

export interface RecordingProcessingState {
  groups: RecordingProcessingGroup[];
  nextCursor: BackgroundTaskCursor | null;
  counts: RecordingProcessingGroupPage['counts'];
  filter: RecordingProcessingFilter;
  loading: boolean;
  status: 'idle' | 'loading' | 'ready' | 'error';
  error: string | null;
  setFilter: (filter: RecordingProcessingFilter) => void;
  refresh: () => Promise<void>;
  loadMore: () => Promise<void>;
  invalidateLibrary: () => void;
}
function matchesFilter(
  group: RecordingProcessingGroup,
  filter: RecordingProcessingFilter,
) {
  const active = ['running', 'queued', 'paused'].includes(group.status);
  return (
    filter === 'all' ||
    (filter === 'active' && active) ||
    (filter === 'history' && !active) ||
    (filter === 'attention' &&
      (['failed', 'completed-with-errors'].includes(group.status) ||
        group.lanes.some(
          (lane) => lane.failed > 0 || lane.applicability === 'protected',
        )))
  );
}
export function createRecordingProcessingStore() {
  let request = 0;
  let refreshQueued = false;
  let historyLoaded = false;
  return create<RecordingProcessingState>((set, get) => {
    async function fetch(append: boolean) {
      if (get().loading) {
        if (!append) refreshQueued = true;
        return;
      }
      const ownedRequest = ++request;
      const current = get();
      if (current.status === 'idle') historyLoaded = false;
      if (append && !current.nextCursor) return;
      set({
        loading: true,
        error: null,
        status: current.groups.length ? 'ready' : 'loading',
      });
      try {
        const page = await listRecordingProcessingGroups({
          filter: current.filter,
          cursor: append ? current.nextCursor : null,
          knownRecordingIds: append
            ? []
            : current.groups.map((group) => group.recordingId),
        });
        if (request !== ownedRequest) return;
        const groups = new Map(
          append
            ? current.groups.map((group) => [group.recordingId, group])
            : page.reconciledGroups
                .filter((group) => matchesFilter(group, current.filter))
                .map((group) => [group.recordingId, group]),
        );
        for (const group of page.groups) groups.set(group.recordingId, group);
        set({
          groups: [...groups.values()].sort(
            (a, b) =>
              b.updatedAtMs - a.updatedAtMs ||
              b.recordingId.localeCompare(a.recordingId),
          ),
          counts: page.counts,
          nextCursor:
            !append && historyLoaded ? current.nextCursor : page.nextCursor,
          status: 'ready',
        });
        if (append) historyLoaded = true;
      } catch (error) {
        if (request === ownedRequest)
          set({
            error: processingFailureSummary(error),
            status: current.groups.length ? 'ready' : 'error',
          });
      } finally {
        if (request === ownedRequest) {
          set({ loading: false });
          if (refreshQueued) {
            refreshQueued = false;
            void fetch(false);
          }
        }
      }
    }
    return {
      groups: [],
      nextCursor: null,
      counts: { all: 0, active: 0, attention: 0, history: 0 },
      filter: 'all',
      loading: false,
      status: 'idle',
      error: null,
      refresh: () => fetch(false),
      loadMore: () => fetch(true),
      invalidateLibrary: () => {
        request++;
        refreshQueued = false;
        historyLoaded = false;
        set({
          groups: [],
          nextCursor: null,
          counts: { all: 0, active: 0, attention: 0, history: 0 },
          loading: false,
          status: 'idle',
          error: null,
        });
      },
      setFilter: (filter) => {
        if (filter === get().filter) return;
        request++;
        refreshQueued = false;
        historyLoaded = false;
        set({
          filter,
          groups: [],
          nextCursor: null,
          loading: false,
          status: 'idle',
          error: null,
        });
        void fetch(false);
      },
    };
  });
}
export const useRecordingProcessingStore = createRecordingProcessingStore();
