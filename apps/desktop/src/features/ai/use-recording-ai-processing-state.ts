import { useCallback, useEffect, useRef, useState } from 'react';
import type { RecordingAiProcessingState } from '@daydream/shared';
import {
  getRecordingAiProcessingState,
  listenToAiProcessingChanged,
} from './ai-client';

type AiProcessingStateStatus = 'error' | 'idle' | 'loading' | 'ready';

export function useRecordingAiProcessingState(recordingId: string): {
  applyState: (state: RecordingAiProcessingState) => void;
  error: string | null;
  reload: () => Promise<RecordingAiProcessingState | null>;
  state: RecordingAiProcessingState | null;
  status: AiProcessingStateStatus;
} {
  const loadRunRef = useRef(0);
  const [state, setState] = useState<RecordingAiProcessingState | null>(null);
  const [status, setStatus] = useState<AiProcessingStateStatus>('idle');
  const [error, setError] = useState<string | null>(null);

  const applyState = useCallback(
    (nextState: RecordingAiProcessingState) => {
      if (nextState.recordingId !== recordingId) {
        return;
      }
      setState(nextState);
      setStatus('ready');
      setError(null);
    },
    [recordingId],
  );

  const reload = useCallback(async () => {
    const loadRunId = loadRunRef.current + 1;
    loadRunRef.current = loadRunId;
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
  }, [applyState, recordingId]);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;
    void listenToAiProcessingChanged((nextState) => {
      if (active) {
        applyState(nextState);
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
      loadRunRef.current += 1;
      unlisten?.();
    };
  }, [applyState]);

  const visibleState = state?.recordingId === recordingId ? state : null;

  return { applyState, error, reload, state: visibleState, status };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
