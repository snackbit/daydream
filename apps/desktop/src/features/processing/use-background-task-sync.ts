import { useEffect } from 'react';
import { useBackgroundTaskStore } from './background-task-store';
import { listenToBackgroundTasksChanged } from './processing-client';

export function useBackgroundTaskSync() {
  const refresh = useBackgroundTaskStore((state) => state.refresh);
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;
    let timer: number | null = null;
    function scheduleRefresh() {
      if (active && timer === null)
        timer = window.setTimeout(() => {
          timer = null;
          void refresh();
        }, 500);
    }
    void refresh();
    void listenToBackgroundTasksChanged(scheduleRefresh)
      .then((cleanup) => {
        if (active) unlisten = cleanup;
        else cleanup();
      })
      .catch(scheduleRefresh);
    // Events are hints. Reconcile after lost events and while long model calls
    // are silent, without rereading captured content.
    const fallback = window.setInterval(scheduleRefresh, 5_000);
    return () => {
      active = false;
      unlisten?.();
      if (timer !== null) window.clearTimeout(timer);
      window.clearInterval(fallback);
    };
  }, [refresh]);
}
