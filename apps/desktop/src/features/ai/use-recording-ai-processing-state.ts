import { useCallback, useEffect, useRef, useState } from 'react';
import type { RecordingAiProcessingState } from '@daydream/shared';
import {
  getRecordingAiProcessingState,
  listenToAiProcessingChanged,
} from './ai-client';

type AiProcessingStateStatus = 'error' | 'idle' | 'loading' | 'ready';

export function useRecordingAiProcessingState(
  recordingId: string,
  visibilityRevision = '',
  range?: { startOffsetMs: number; endOffsetMs: number; ready?: boolean },
): {
  applyState: (state: RecordingAiProcessingState) => void;
  error: string | null;
  reload: () => Promise<RecordingAiProcessingState | null>;
  state: RecordingAiProcessingState | null;
  status: AiProcessingStateStatus;
  refreshing: boolean;
} {
  const loadRunRef = useRef(0);
  const [state, setState] = useState<{
    data: RecordingAiProcessingState;
    revision: string;
  } | null>(null);
  const [status, setStatus] = useState<AiProcessingStateStatus>('idle');
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [eventsUnavailable, setEventsUnavailable] = useState(false);

  const applyState = useCallback(
    (nextState: RecordingAiProcessingState) => {
      if (nextState.recordingId !== recordingId) {
        return;
      }
      loadRunRef.current += 1;
      setState({ data: nextState, revision: visibilityRevision });
      setStatus('ready');
      setRefreshing(false);
      setError(null);
    },
    [recordingId, visibilityRevision],
  );

  const reload = useCallback(async () => {
    if (range?.ready === false) return null;
    const loadRunId = loadRunRef.current + 1;
    loadRunRef.current = loadRunId;
    setRefreshing(true);
    setState((current) =>
      current?.data.recordingId === recordingId &&
      current.revision === visibilityRevision
        ? current
        : null,
    );
    setStatus((current) => (current === 'ready' ? current : 'loading'));
    setError(null);
    try {
      const nextState = await getRecordingAiProcessingState({
        recordingId,
        ...range,
      });
      if (loadRunRef.current !== loadRunId) {
        return null;
      }
      applyState(nextState);
      return nextState;
    } catch (loadError) {
      if (loadRunRef.current !== loadRunId) {
        return null;
      }
      setError(errorMessage(loadError));
      setStatus('error');
      setRefreshing(false);
      return null;
    }
  }, [applyState, recordingId, visibilityRevision, range]);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;
    let refreshTimer: number | null = null;
    void listenToAiProcessingChanged((nextState) => {
      if ('sourceKind' in nextState && nextState.sourceKind === 'waveform')
        return;
      if (active && 'chunks' in nextState) {
        applyState(nextState);
      } else if (active && nextState.recordingId === recordingId) {
        if (refreshTimer === null)
          refreshTimer = window.setTimeout(() => {
            refreshTimer = null;
            void reload();
          }, 500);
      }
    })
      .then((nextUnlisten) => {
        if (active) {
          setEventsUnavailable(false);
          unlisten = nextUnlisten;
        } else {
          nextUnlisten();
        }
      })
      .catch(() => {
        // Reconciliation remains available if event delivery is unavailable.
        if (active) setEventsUnavailable(true);
      });

    return () => {
      active = false;
      if (refreshTimer !== null) window.clearTimeout(refreshTimer);
      loadRunRef.current += 1;
      unlisten?.();
    };
  }, [applyState, recordingId, reload]);

  const visibleState =
    state?.data.recordingId === recordingId &&
    state.revision === visibilityRevision
      ? state.data
      : null;
  const hasPendingWork =
    visibleState?.job?.status === 'queued' ||
    visibleState?.job?.status === 'running' ||
    visibleState?.chunks.some(
      (chunk) => chunk.status === 'queued' || chunk.status === 'running',
    );
  useEffect(() => {
    let busy = false;
    async function reconcile() {
      if (busy) return;
      busy = true;
      try {
        await reload();
      } finally {
        busy = false;
      }
    }
    const focus = () => {
      void reconcile();
    };
    const timer =
      hasPendingWork || eventsUnavailable
        ? window.setInterval(focus, 5000)
        : undefined;
    window.addEventListener('focus', focus);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener('focus', focus);
    };
  }, [hasPendingWork, eventsUnavailable, reload]);

  return { applyState, error, reload, state: visibleState, status, refreshing };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
