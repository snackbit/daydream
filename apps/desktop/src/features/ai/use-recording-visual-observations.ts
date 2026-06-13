import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  RecordingAiProcessingState,
  RecordingVisualObservations,
} from '@daydream/shared';
import { getRecordingVisualObservations } from './ai-client';

type VisualObservationsStatus = 'error' | 'idle' | 'loading' | 'ready';

export function useRecordingVisualObservations(
  recordingId: string,
  aiProcessingState: RecordingAiProcessingState | null,
): {
  error: string | null;
  reload: () => Promise<RecordingVisualObservations | null>;
  state: RecordingVisualObservations | null;
  status: VisualObservationsStatus;
} {
  const loadRunRef = useRef(0);
  const [state, setState] = useState<RecordingVisualObservations | null>(null);
  const [status, setStatus] = useState<VisualObservationsStatus>('idle');
  const [error, setError] = useState<string | null>(null);
  const visualChunkSignature = useMemo(
    () => visualProcessingSignature(aiProcessingState),
    [aiProcessingState],
  );

  const reload = useCallback(async () => {
    const loadRunId = loadRunRef.current + 1;
    loadRunRef.current = loadRunId;
    setStatus((current) => (current === 'ready' ? current : 'loading'));
    setError(null);
    try {
      const nextState = await getRecordingVisualObservations({ recordingId });
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
  }, [reload, visualChunkSignature]);

  useEffect(
    () => () => {
      loadRunRef.current += 1;
    },
    [],
  );

  const visibleState = state?.recordingId === recordingId ? state : null;

  return { error, reload, state: visibleState, status };
}

function visualProcessingSignature(
  aiProcessingState: RecordingAiProcessingState | null,
): string {
  return (
    aiProcessingState?.chunks
      .filter(
        (chunk) => chunk.sourceKind === 'ocr' || chunk.sourceKind === 'vlm',
      )
      .map(
        (chunk) =>
          `${chunk.id}:${chunk.sourceKind}:${chunk.status}:${chunk.updatedAtMs}:${chunk.errorCode ?? ''}`,
      )
      .join('|') ?? ''
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
