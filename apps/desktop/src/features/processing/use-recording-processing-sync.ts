import { useEffect } from 'react';
import { listenToBackgroundTasksChanged } from './processing-client';
import {
  useRecordingProcessingStore,
  type RecordingProcessingStore,
} from './recording-processing-store';

export function useRecordingProcessingSync(
  useStore: RecordingProcessingStore = useRecordingProcessingStore,
  enabled = true,
) {
  const refresh = useStore((state) => state.refresh);
  useEffect(() => {
    if (!enabled) return;
    let active = true;
    let cleanup: (() => void) | null = null;
    let timer: number | null = null;
    let eventsUnavailable = false;
    function reconcile() {
      if (active && timer === null)
        timer = window.setTimeout(() => {
          timer = null;
          void refresh();
        }, 500);
    }
    void refresh();
    void listenToBackgroundTasksChanged(reconcile)
      .then((unlisten) => {
        if (active) cleanup = unlisten;
        else unlisten();
      })
      .catch(() => {
        eventsUnavailable = true;
        reconcile();
      });
    const fallback = window.setInterval(() => {
      if (
        eventsUnavailable ||
        useStore.getState().counts.active ||
        useStore.getState().status === 'error'
      )
        reconcile();
    }, 5000);
    window.addEventListener('focus', reconcile);
    return () => {
      active = false;
      cleanup?.();
      if (timer !== null) window.clearTimeout(timer);
      window.clearInterval(fallback);
      window.removeEventListener('focus', reconcile);
    };
  }, [refresh, enabled, useStore]);
}
