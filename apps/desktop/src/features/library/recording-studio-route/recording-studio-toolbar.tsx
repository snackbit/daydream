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
    (processingJob?.status === 'cancelled' ||
      processingJob?.status === 'failed');
  const processAiLabel = processingStartBusy
    ? 'Starting...'
    : processingActive
      ? 'Processing...'
      : processingSucceeded
        ? 'Processed'
        : processingRetryable
          ? 'Retry AI'
          : 'Process AI';

  return (
    <div className="flex shrink-0 flex-col gap-3 border-b px-4 py-3 xl:flex-row xl:items-center xl:justify-between">
      <div className="min-w-0">
        <h1 className="text-sm font-semibold tracking-normal">
          {detail ? recordingTitle(detail) : 'Recording'}
        </h1>
        <div className="text-muted-foreground mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-[11px]">
          {detail ? (
            <>
              <span>{availabilityLabel(detail.availability)}</span>
              {detail.title ? (
                <span>{formatTimestamp(detail.startedAtMs)}</span>
              ) : null}
              <span>{formatDuration(detail.durationMs)}</span>
              <span>{detail.segments.length} segments</span>
            </>
          ) : (
            <span>
              {loadStatus === 'error'
                ? 'Recording unavailable'
                : 'Loading recording...'}
            </span>
          )}
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2 xl:justify-end">
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
  );
}
