import { useCallback, useEffect, useRef, useState } from 'react';

// Visibility belongs to the recording/redaction revision that produced a read.
// A changed key hides cached data during render, before effects or IPC settle.
export function useRecordingResult<Result extends { recordingId: string }>({
  recordingId,
  visibilityRevision,
  processingSignature,
  read,
  range,
}: {
  recordingId: string;
  visibilityRevision: string;
  processingSignature: string;
  read: (input: {
    recordingId: string;
    startOffsetMs?: number;
    endOffsetMs?: number;
  }) => Promise<Result>;
  range?: { startOffsetMs: number; endOffsetMs: number };
}) {
  const requestRef = useRef(0);
  const [result, setResult] = useState<{
    recordingId: string;
    revision: string;
    data: Result;
  } | null>(null);
  const [status, setStatus] = useState<'error' | 'idle' | 'loading' | 'ready'>(
    'idle',
  );
  const [error, setError] = useState<string | null>(null);
  const reload = useCallback(async () => {
    const request = ++requestRef.current;
    setResult((current) =>
      current?.recordingId === recordingId &&
      current.revision === visibilityRevision
        ? current
        : null,
    );
    setStatus('loading');
    setError(null);
    try {
      const data = await read({ recordingId, ...range });
      if (requestRef.current !== request) return null;
      setResult(
        data.recordingId === recordingId
          ? { recordingId, revision: visibilityRevision, data }
          : null,
      );
      setStatus('ready');
      return data;
    } catch (failure) {
      if (requestRef.current !== request) return null;
      setError(failure instanceof Error ? failure.message : String(failure));
      setStatus('error');
      return null;
    }
  }, [read, recordingId, visibilityRevision, range]);
  useEffect(() => {
    void reload();
    return () => {
      requestRef.current += 1;
    };
  }, [reload, processingSignature]);
  const state =
    result?.recordingId === recordingId &&
    result.revision === visibilityRevision
      ? result.data
      : null;
  return { error, reload, state, status };
}
