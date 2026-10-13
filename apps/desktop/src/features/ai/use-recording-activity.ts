import { useMemo } from 'react';
import type {
  RecordingActivity,
  RecordingAiProcessingState,
} from '@daydream/shared';
import { getRecordingActivity } from './ai-client';
import { useRecordingResult } from './results/use-recording-result';

type RecordingActivityStatus = 'error' | 'idle' | 'loading' | 'ready';

export function useRecordingActivity(
  recordingId: string,
  aiProcessingState: RecordingAiProcessingState | null,
  redactionSignature = '',
  range?: { startOffsetMs: number; endOffsetMs: number },
): {
  error: string | null;
  reload: () => Promise<RecordingActivity | null>;
  state: RecordingActivity | null;
  status: RecordingActivityStatus;
} {
  const activityChunkSignature = useMemo(
    () => activityProcessingSignature(aiProcessingState),
    [aiProcessingState],
  );

  return useRecordingResult({
    recordingId,
    visibilityRevision: redactionSignature,
    processingSignature: activityChunkSignature,
    read: getRecordingActivity,
    ...(range ? { range } : {}),
  });
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
