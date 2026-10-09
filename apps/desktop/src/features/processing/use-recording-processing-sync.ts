import { useEffect } from 'react';
import { listenToBackgroundTasksChanged } from './processing-client';
import { useRecordingProcessingStore } from './recording-processing-store';

export function useRecordingProcessingSync() {
  const refresh = useRecordingProcessingStore((state) => state.refresh);
  useEffect(() => {
    let active = true;
    let cleanup: (() => void) | null = null;
    let timer: number | null = null;
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
      .catch(reconcile);
    const fallback = window.setInterval(reconcile, 5000);
    return () => {
      active = false;
      cleanup?.();
      if (timer !== null) window.clearTimeout(timer);
      window.clearInterval(fallback);
    };
  }, [refresh]);
}
