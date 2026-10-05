import { useCallback, useState } from 'react';
import type { OcrDocument, ProcessingJob } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import {
  retryProcessingJob,
  resumeProcessingJob,
} from '@/features/ai/ai-client';
import { reprocessRecordingOcr } from '@/features/ai/ocr/ocr-client';
import { OcrTextPanel } from './ocr-text-panel';

export interface OcrDocumentStateProps {
  document: OcrDocument | null;
  job: ProcessingJob | null;
  loading: boolean;
  error: string | null;
  recordingId: string;
  onRetryRead: () => void;
}
export function OcrDocumentState({
  document,
  job,
  loading,
  error,
  recordingId,
  onRetryRead,
}: OcrDocumentStateProps) {
  const [retryStatus, setRetryStatus] = useState<
    'idle' | 'working' | 'started' | 'error'
  >('idle');
  const [retryError, setRetryError] = useState<string | null>(null);
  const retryProcessing = useCallback(async () => {
    setRetryStatus('working');
    setRetryError(null);
    try {
      if (job?.status === 'paused')
        await resumeProcessingJob({ jobId: job.id });
      else if (job && (job.status === 'failed' || job.status === 'cancelled'))
        await retryProcessingJob({ jobId: job.id });
      else await reprocessRecordingOcr(recordingId);
      setRetryStatus('started');
      onRetryRead();
    } catch (failure) {
      setRetryStatus('error');
      setRetryError(
        failure instanceof Error
          ? failure.message
          : typeof failure === 'object' &&
              failure !== null &&
              'message' in failure &&
              typeof failure.message === 'string'
            ? failure.message
            : 'OCR processing could not be restarted.',
      );
    }
  }, [job, onRetryRead, recordingId]);
  if (loading && !document && !job)
    return (
      <p role="status" className="text-muted-foreground text-[11px]">
        Loading OCR text…
      </p>
    );
  if (error)
    return (
      <div role="alert" className="text-destructive text-[11px]">
        {error}
        <Button onClick={onRetryRead} size="sm" variant="outline">
          Retry loading OCR
        </Button>
      </div>
    );
  // The backend returns only eligible current documents. A mixed parent may fail
  // on audio while this source's OCR succeeded; that literal text stays readable.
  if (document?.status === 'succeeded')
    return document.text.trim() ? (
      <OcrTextPanel document={document} key={document.id} />
    ) : (
      <p className="text-muted-foreground text-[11px]">
        No text detected in this frame.
      </p>
    );
  const failed = document?.status === 'failed' || job?.status === 'failed';
  const resumable = job?.status === 'paused' || job?.status === 'cancelled';
  if (failed || resumable)
    return (
      <div className="space-y-2">
        <p
          role={failed ? 'alert' : 'status'}
          className={
            failed
              ? 'text-destructive text-[11px]'
              : 'text-muted-foreground text-[11px]'
          }
        >
          {failed
            ? 'OCR processing failed for this source.'
            : job?.status === 'paused'
              ? 'OCR processing is paused.'
              : 'OCR processing was cancelled.'}
          {document?.errorCode || job?.errorCode
            ? ` (${document?.errorCode ?? job?.errorCode})`
            : ''}
        </p>
        <p className="text-muted-foreground text-[10px]">
          Retry uses the saved OCR configuration. Vision OCR may send eligible
          frames to your configured provider.
        </p>
        <Button
          disabled={retryStatus === 'working' || retryStatus === 'started'}
          onClick={retryProcessing}
          size="sm"
          variant="outline"
        >
          {retryStatus === 'working'
            ? 'Starting OCR…'
            : retryStatus === 'started'
              ? 'OCR processing queued'
              : job?.status === 'paused'
                ? 'Resume processing'
                : 'Retry processing'}
        </Button>
        {retryError ? (
          <p role="alert" className="text-destructive text-[11px]">
            {retryError}
          </p>
        ) : null}
      </div>
    );
  if (job?.status === 'queued' || job?.status === 'running')
    return (
      <p role="status" className="text-muted-foreground text-[11px]">
        {job.status === 'queued'
          ? 'OCR processing is queued.'
          : 'OCR processing is running.'}{' '}
        No document is available for this sample yet.
      </p>
    );
  if (document?.status === 'gap')
    return (
      <p className="text-muted-foreground text-[11px]">
        OCR is unavailable for this sample. Select another eligible frame.
      </p>
    );
  return (
    <p className="text-muted-foreground text-[11px]">
      No OCR document for this frame. Processing may be pending.
    </p>
  );
}
