import { useCallback, useRef, useState } from 'react';
import type { ControlRecordingProcessingGroupsInput } from '@daydream/shared';
import { controlRecordingProcessingGroups } from './processing-client';
import {
  processingFailureSummary,
  describeProcessingError,
} from './processing-errors';
import { useRecordingProcessingStore } from './recording-processing-store';

export function useRecordingProcessingActions(recordingId: string) {
  const refresh = useRecordingProcessingStore((state) => state.refresh);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const admission = useRef(false);
  const perform = useCallback(
    async (
      input: Omit<ControlRecordingProcessingGroupsInput, 'recordingId'>,
    ) => {
      if (admission.current) return;
      admission.current = true;
      setPending(true);
      setError(null);
      try {
        const result = await controlRecordingProcessingGroups({
          ...input,
          recordingId,
        });
        const failed = result.results.filter((result) => !result.succeeded);
        if (failed.length)
          setError(
            `${failed.length} processing ${failed.length === 1 ? 'job needs' : 'jobs need'} attention. ${describeProcessingError(failed[0]?.errorCode ?? '').summary}`,
          );
        else if (!result.results.length)
          setError(
            'This action is no longer available. The recording activity has been refreshed.',
          );
        await refresh();
      } catch (error) {
        setError(processingFailureSummary(error));
        await refresh();
      } finally {
        admission.current = false;
        setPending(false);
      }
    },
    [recordingId, refresh],
  );
  return { pending, error, perform };
}
