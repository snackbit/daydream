import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  RecordingActivity,
  RecordingAiProcessingState,
} from '@daydream/shared';
import { getRecordingActivity } from './ai-client';

type RecordingActivityStatus = 'error' | 'idle' | 'loading' | 'ready';

export function useRecordingActivity(
  recordingId: string,
  aiProcessingState: RecordingAiProcessingState | null,
  redactionSignature = '',
): {
  error: string | null;
  reload: () => Promise<RecordingActivity | null>;
  state: RecordingActivity | null;
  status: RecordingActivityStatus;
} {
  const loadRunRef = useRef(0);
  const [state, setState] = useState<RecordingActivity | null>(null);
  const [status, setStatus] = useState<RecordingActivityStatus>('idle');
  const [error, setError] = useState<string | null>(null);
  const activityChunkSignature = useMemo(
    () => activityProcessingSignature(aiProcessingState),
    [aiProcessingState],
  );

  const reload = useCallback(async () => {
    const loadRunId = loadRunRef.current + 1;
    loadRunRef.current = loadRunId;
    setStatus((current) => (current === 'ready' ? current : 'loading'));
    setError(null);
    try {
      const nextState = await getRecordingActivity({ recordingId });
      if (loadRunRef.current !== loadRunId) {
        return null;
      }
      setState(nextState.recordingId === recordingId ? nextState : null);
      setStatus('ready');
      return nextState;
    } catch (loadError) {
      if (loadRunRef.current !== loadRunId) {
        return null;
      }
      setError(errorMessage(loadError));
      setStatus('error');
      return null;
    }
  }, [recordingId]);

  useEffect(() => {
    void reload();
  }, [activityChunkSignature, redactionSignature, reload]);

  useEffect(
    () => () => {
      loadRunRef.current += 1;
    },
    [],
  );

  const visibleState = state?.recordingId === recordingId ? state : null;

  return { error, reload, state: visibleState, status };
}

function activityProcessingSignature(
  aiProcessingState: RecordingAiProcessingState | null,
): string {
  return (
    aiProcessingState?.chunks
      .filter((chunk) => chunk.sourceKind === 'activity')
      .map(
        (chunk) =>
          `${chunk.id}:${chunk.status}:${chunk.updatedAtMs}:${chunk.errorCode ?? ''}`,
      )
      .join('|') ?? ''
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
