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
): {
  applyState: (state: RecordingAiProcessingState) => void;
  error: string | null;
  reload: () => Promise<RecordingAiProcessingState | null>;
  state: RecordingAiProcessingState | null;
  status: AiProcessingStateStatus;
} {
  const loadRunRef = useRef(0);
  const [state, setState] = useState<{
    data: RecordingAiProcessingState;
    revision: string;
  } | null>(null);
  const [status, setStatus] = useState<AiProcessingStateStatus>('idle');
  const [error, setError] = useState<string | null>(null);

  const applyState = useCallback(
    (nextState: RecordingAiProcessingState) => {
      if (nextState.recordingId !== recordingId) {
        return;
      }
      loadRunRef.current += 1;
      setState({ data: nextState, revision: visibilityRevision });
      setStatus('ready');
      setError(null);
    },
    [recordingId, visibilityRevision],
  );

  const reload = useCallback(async () => {
    const loadRunId = loadRunRef.current + 1;
    loadRunRef.current = loadRunId;
    setState((current) =>
      current?.data.recordingId === recordingId &&
      current.revision === visibilityRevision
        ? current
        : null,
    );
    setStatus((current) => (current === 'ready' ? current : 'loading'));
    setError(null);
    try {
      const nextState = await getRecordingAiProcessingState({ recordingId });
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
      return null;
    }
  }, [applyState, recordingId, visibilityRevision]);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;
    let refreshTimer: number | null = null;
    void listenToAiProcessingChanged((nextState) => {
      if (active && 'chunks' in nextState) {
        applyState(nextState);
      } else if (active && nextState.recordingId === recordingId) {
        if (refreshTimer === null)
          refreshTimer = window.setTimeout(() => {
            refreshTimer = null;
            void reload();
          }, 500);
      }
    }).then((nextUnlisten) => {
      if (active) {
        unlisten = nextUnlisten;
      } else {
        nextUnlisten();
      }
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

  return { applyState, error, reload, state: visibleState, status };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
