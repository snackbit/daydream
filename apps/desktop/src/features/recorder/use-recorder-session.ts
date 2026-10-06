import { useEffect } from 'react';
import { recorderLifecycleLocked } from './recorder-lifecycle-state';
import { useRecorderWorkspaceStore } from './recorder-workspace-store';

export function useRecorderSession() {
  const status = useRecorderWorkspaceStore((state) => state.status);
  const pending = useRecorderWorkspaceStore(
    (state) => state.pendingLifecycleAction,
  );
  const stats = useRecorderWorkspaceStore((state) => state.recordingStats);
  const error = useRecorderWorkspaceStore((state) => state.error);
  const pause = useRecorderWorkspaceStore((state) => state.pause);
  const resume = useRecorderWorkspaceStore((state) => state.resume);
  const stop = useRecorderWorkspaceStore((state) => state.stop);
  const subscribe = useRecorderWorkspaceStore(
    (state) => state.subscribeToStatusEvents,
  );
  const refreshStats = useRecorderWorkspaceStore(
    (state) => state.refreshRecordingStats,
  );
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;
    void subscribe()
      .then((cleanup) => {
        if (active) unlisten = cleanup;
        else cleanup();
      })
      .catch(() => undefined);
    return () => {
      active = false;
      unlisten?.();
    };
  }, [subscribe]);
  useEffect(() => {
    if (status?.state !== 'recording' && status?.state !== 'paused') return;
    let active = true;
    let timer: number | null = null;
    async function refresh() {
      await refreshStats();
      if (active) timer = window.setTimeout(refresh, 1_000);
    }
    void refresh();
    return () => {
      active = false;
      if (timer !== null) window.clearTimeout(timer);
    };
  }, [refreshStats, status?.activeRecordingId, status?.state]);
  return {
    status,
    stats,
    error,
    pause,
    resume,
    stop,
    locked: recorderLifecycleLocked(pending, status),
  };
}
