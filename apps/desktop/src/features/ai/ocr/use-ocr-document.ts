import { useCallback, useEffect, useEffectEvent, useState } from 'react';
import type { GetOcrDocumentInput } from '@daydream/shared';
import { listenToAiProcessingChanged } from '../ai-client';
import { useRecordingResult } from '../results/use-recording-result';
import { getOcrDocument } from './ocr-client';

export function useOcrDocument(
  input: GetOcrDocumentInput,
  privacyRevision: string,
  processingRevision = '',
) {
  const { documentId, recordingId, sourceTrackId, timestampMs } = input;
  const read = useCallback(
    () =>
      getOcrDocument({
        recordingId,
        sourceTrackId,
        timestampMs,
        documentId: documentId ?? null,
      }),
    [documentId, recordingId, sourceTrackId, timestampMs],
  );
  const request = useRecordingResult({
    recordingId,
    processingSignature: processingRevision,
    read,
    visibilityRevision: JSON.stringify([
      privacyRevision,
      processingRevision,
      sourceTrackId,
      timestampMs,
      documentId ?? null,
    ]),
  });
  const [updateError, setUpdateError] = useState<string | null>(null);
  const refreshAfterProcessing = useEffectEvent(() => {
    void request.reload();
  });
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;
    void listenToAiProcessingChanged((state) => {
      if (active && state.recordingId === recordingId) refreshAfterProcessing();
    })
      .then((cleanup) => {
        if (active) {
          unlisten = cleanup;
          setUpdateError(null);
        } else cleanup();
      })
      .catch(() => {
        if (active)
          setUpdateError(
            'Live OCR updates are unavailable. Refresh to read current results.',
          );
      });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [recordingId]);
  return {
    ...request,
    document: request.state?.document ?? null,
    job: request.state?.job ?? null,
    updateError,
  };
}
