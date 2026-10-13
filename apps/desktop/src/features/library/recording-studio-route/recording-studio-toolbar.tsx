import type { ReactNode } from 'react';
import type {
  LibraryRecordingDetail,
  RecordingAiProcessingState,
} from '@daydream/shared';
import {
  Bug,
  Loader2,
  Pencil,
  RefreshCw,
  Sparkles,
  Trash2,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ProcessingError } from '@/features/processing/components/processing-error';
import { StudioProcessingOverview } from '@/features/processing/components/studio-processing-overview';
import { LibraryRecoveryAction } from '../components/library-recovery-action';
import {
  availabilityLabel,
  formatDuration,
  formatTimestamp,
} from '../library-format';
import {
  isAiProcessingStartable,
  isRecordingDeletable,
  isRecordingRenameable,
  recordingTitle,
} from './recording-studio-route-utils';

export function RecordingStudioToolbar({
  aiProcessing,
  debugEnabled,
  detail,
  loadStatus,
  onDelete,
  onDebug,
  onRefresh,
  onRename,
  onStartAiProcessing,
  processingStartBusy,
  loadingStatus = null,
  title,
  navigation,
  processingRecordingIds,
}: {
  aiProcessing: { state: RecordingAiProcessingState | null };
  debugEnabled: boolean;
  detail: LibraryRecordingDetail | null;
  loadStatus: 'error' | 'idle' | 'loading' | 'ready';
  onDelete: () => void;
  onDebug: () => void;
  onRefresh: () => void;
  onRename: () => void;
  onStartAiProcessing: () => void;
  processingStartBusy: boolean;
  loadingStatus?: string | null;
  title?: string;
  navigation?: ReactNode;
  processingRecordingIds?: readonly string[];
}) {
  const processingJob = aiProcessing.state?.job ?? null;
  const hasActiveProcessingChunks =
    aiProcessing.state?.chunks.some(
      (chunk) => chunk.status === 'queued' || chunk.status === 'running',
    ) ?? false;
  const processingActive =
    processingJob?.status === 'queued' ||
    processingJob?.status === 'running' ||
    hasActiveProcessingChunks;
  const processingSucceeded =
    processingJob?.status === 'succeeded' && !hasActiveProcessingChunks;
  const processingRetryable =
    !hasActiveProcessingChunks &&
    (processingJob?.status === 'completed-with-errors' ||
      processingJob?.status === 'failed');
  const processAiLabel = processingStartBusy
    ? 'Starting...'
    : processingActive
      ? 'Processing...'
      : processingSucceeded
        ? 'Processed'
        : processingRetryable
          ? 'Retry failed sections'
          : processingJob?.status === 'paused'
            ? 'Resume processing'
            : processingJob?.status === 'cancelled'
              ? 'Restart cancelled processing'
              : 'Process recording';
  const failures = new Map<string, string>();
  for (const chunk of aiProcessing.state?.chunks ?? []) {
    if (
      (chunk.status === 'failed' || chunk.status === 'completed-with-errors') &&
      chunk.errorCode
    )
      failures.set(chunk.sourceKind, chunk.errorCode);
  }
  if (!failures.size && processingJob?.errorCode)
    failures.set('Processing', processingJob.errorCode);

  return (
    <div className="shrink-0">
      <div
        data-day={title !== undefined}
        className="flex flex-col gap-6 px-5 py-6 lg:px-8 lg:py-8 lg:data-[day=true]:flex-row lg:data-[day=true]:items-center lg:data-[day=true]:justify-between lg:data-[day=true]:py-5 xl:flex-row xl:items-center xl:justify-between"
      >
        <div className="min-w-0">
          <h1 className="text-[32px] font-light tracking-tight lg:text-4xl">
            {title ?? (detail ? recordingTitle(detail) : 'Recording')}
          </h1>
          {navigation ? <div className="mt-3">{navigation}</div> : null}
          <div className="text-muted-foreground mt-3 flex flex-wrap items-center gap-x-3 gap-y-2 text-xs">
            {detail ? (
              <>
                {title ? (
                  <span>
                    {detail.title ?? formatTimestamp(detail.startedAtMs)}
                  </span>
                ) : null}
                <span>{availabilityLabel(detail.availability)}</span>
                {detail.title ? (
                  <span>{formatTimestamp(detail.startedAtMs)}</span>
                ) : null}
                <span>{formatDuration(detail.durationMs)}</span>
              </>
            ) : (
              <span>
                {loadStatus === 'error'
                  ? 'Recording unavailable'
                  : 'Loading recording...'}
              </span>
            )}
          </div>
          <p
            role="status"
            className="text-muted-foreground mt-2 h-4 truncate text-xs leading-4"
            title={detail ? (loadingStatus ?? undefined) : undefined}
          >
            {detail ? loadingStatus : null}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2 xl:justify-end">
          {detail ? (
            <StudioProcessingOverview
              recordingIds={processingRecordingIds ?? [detail.id]}
            />
          ) : null}
          {detail ? (
            <LibraryRecoveryAction key={detail.id} detail={detail} />
          ) : null}
          {detail ? (
            <Button
              className="gap-2"
              disabled={
                !isAiProcessingStartable(detail) ||
                processingStartBusy ||
                processingActive ||
                processingSucceeded
              }
              variant="outline"
              onClick={onStartAiProcessing}
            >
              {processingStartBusy || processingActive ? (
                <Loader2 className="size-3.5 animate-spin" />
              ) : (
                <Sparkles className="size-3.5" />
              )}
              {processAiLabel}
            </Button>
          ) : null}
          {detail && debugEnabled ? (
            <Button className="gap-2" variant="outline" onClick={onDebug}>
              <Bug className="size-3.5" />
              Debug
            </Button>
          ) : null}
          {detail && isRecordingRenameable(detail) ? (
            <Button className="gap-2" variant="outline" onClick={onRename}>
              <Pencil className="size-3.5" />
              Rename
            </Button>
          ) : null}
          {detail && isRecordingDeletable(detail) ? (
            <Button className="gap-2" variant="destructive" onClick={onDelete}>
              <Trash2 className="size-3.5" />
              Delete
            </Button>
          ) : null}
          <Button className="gap-2" variant="outline" onClick={onRefresh}>
            <RefreshCw className="size-3.5" />
            Refresh
          </Button>
        </div>
      </div>
      {failures.size ? (
        <div
          aria-label="Processing failures"
          className="grid gap-2 border-b px-5 pb-4 lg:px-8"
        >
          {[...failures].map(([section, code]) => (
            <div key={section}>
              <p className="text-xs font-medium capitalize">{section}</p>
              <ProcessingError errorCode={code} technicalDetails={false} />
            </div>
          ))}
        </div>
      ) : processingJob?.status === 'paused' ||
        processingJob?.status === 'cancelled' ? (
        <p role="status" className="px-5 pb-4 text-xs lg:px-8">
          Processing {processingJob.status}. Saved results are retained.
        </p>
      ) : null}
    </div>
  );
}
