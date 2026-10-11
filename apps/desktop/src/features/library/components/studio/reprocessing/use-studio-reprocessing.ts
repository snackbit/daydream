import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  PreviewRecordingReprocessingInput,
  RecordingReprocessingPreview,
  ReprocessingOperation,
  ReprocessingTarget,
  ProcessingJob,
  RecordingReprocessingScope,
} from '@daydream/shared';
import {
  previewRecordingReprocessing,
  startRecordingReprocessing,
} from './reprocessing-client';

export function useStudioReprocessing(
  recordingId: string,
  observedJob: ProcessingJob | null = null,
) {
  const [owner, setOwner] = useState(recordingId);
  const [request, setRequest] =
    useState<PreviewRecordingReprocessingInput | null>(null);
  const [preview, setPreview] = useState<RecordingReprocessingPreview | null>(
    null,
  );
  const [pending, setPending] = useState(false),
    [error, setError] = useState<string | null>(null),
    [queued, setQueued] = useState(false);
  const [submitted, setSubmitted] = useState<{
    recordingId: string;
    job: ProcessingJob;
    scopes: RecordingReprocessingScope[];
  } | null>(null);
  const generation = useRef(0);
  const open = useCallback(
    (targets: ReprocessingTarget[]) => {
      generation.current++;
      setOwner(recordingId);
      setPending(false);
      setRequest({ recordingId, targets });
      setPreview(null);
      setError(null);
      setQueued(false);
    },
    [recordingId],
  );
  const close = useCallback(() => {
    if (pending) return;
    generation.current++;
    setRequest(null);
    setPreview(null);
    setError(null);
  }, [pending]);
  const operations = useCallback((selected: ReprocessingOperation[]) => {
    setRequest((current) =>
      current ? { ...current, operations: selected } : null,
    );
    setPreview(null);
    setError(null);
  }, []);
  useEffect(() => {
    if (!request || request.recordingId !== recordingId) return;
    const run = ++generation.current;
    void previewRecordingReprocessing(request)
      .then((result) => {
        if (generation.current === run) setPreview(result);
      })
      .catch(() => {
        if (generation.current === run)
          setError(
            'Could not prepare reprocessing. Refresh the recording and try again.',
          );
      });
    return () => {
      if (generation.current === run) generation.current++;
    };
  }, [request, recordingId]);
  const start = useCallback(async () => {
    if (!request || !preview || pending || preview.blockedReason) return;
    const run = generation.current;
    setPending(true);
    setError(null);
    try {
      const job = await startRecordingReprocessing({
        ...request,
        planFingerprint: preview.planFingerprint,
      });
      if (run === generation.current) {
        setSubmitted({ recordingId, job, scopes: preview.scopes });
        setQueued(true);
        setRequest(null);
        setPreview(null);
      }
    } catch {
      if (run === generation.current)
        setError(
          'Reprocessing could not start. Settings, eligibility, or active work may have changed. Close and reopen this dialog to review the current plan.',
        );
    } finally {
      if (run === generation.current) setPending(false);
    }
  }, [request, preview, pending, recordingId]);
  // Keep admitted work visible until the native read model catches up. A newer
  // persisted job also owns this handoff, so an old submission cannot reappear.
  const submittedVisible =
    submitted?.recordingId === recordingId &&
    !(
      observedJob &&
      (observedJob.id === submitted.job.id ||
        observedJob.createdAtMs > submitted.job.createdAtMs)
    );
  const plannedScopes = useMemo(
    () =>
      owner !== recordingId
        ? []
        : pending && preview
          ? preview.scopes.map((scope) => ({
              ...scope,
              status: 'planned' as const,
            }))
          : submittedVisible
            ? submitted.scopes.map((scope) => ({
                ...scope,
                status: 'queued' as const,
              }))
            : [],
    [owner, recordingId, pending, preview, submittedVisible, submitted],
  );
  return {
    plannedScopes,
    open,
    close,
    operations,
    start,
    request: request?.recordingId === recordingId ? request : null,
    preview: owner === recordingId ? preview : null,
    pending: owner === recordingId && pending,
    error: owner === recordingId ? error : null,
    queued: owner === recordingId && queued,
  };
}
export type StudioReprocessingController = ReturnType<
  typeof useStudioReprocessing
>;
