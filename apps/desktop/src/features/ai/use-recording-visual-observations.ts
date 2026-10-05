import { useMemo } from 'react';
import type {
  RecordingAiProcessingState,
  RecordingVisualObservations,
} from '@daydream/shared';
import { getRecordingVisualObservations } from './ai-client';
import { useRecordingResult } from './results/use-recording-result';

type VisualObservationsStatus = 'error' | 'idle' | 'loading' | 'ready';

export function useRecordingVisualObservations(
  recordingId: string,
  aiProcessingState: RecordingAiProcessingState | null,
  redactionSignature = '',
): {
  error: string | null;
  reload: () => Promise<RecordingVisualObservations | null>;
  state: RecordingVisualObservations | null;
  status: VisualObservationsStatus;
} {
  const visualChunkSignature = useMemo(
    () => visualProcessingSignature(aiProcessingState),
    [aiProcessingState],
  );

  return useRecordingResult({
    recordingId,
    visibilityRevision: redactionSignature,
    processingSignature: visualChunkSignature,
    read: getRecordingVisualObservations,
  });
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
