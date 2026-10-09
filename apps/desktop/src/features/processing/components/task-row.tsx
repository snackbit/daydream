import { useCallback, useState } from 'react';
import { Link } from '@tanstack/react-router';
import type { BackgroundTask } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import {
  pauseProcessingJob,
  resumeProcessingJob,
  cancelProcessingJob,
  downloadAiModel,
  cancelAiModelDownload,
} from '@/features/ai/ai-client';
import { controlRecallJob } from '@/features/recall/recall-client';
import { useBackgroundTaskStore } from '../background-task-store';
import {
  retryProcessingUnits,
  reprocessRecordingTranscription,
} from '../processing-client';
import {
  describeProcessingError,
  processingFailureSummary,
} from '../processing-errors';
import {
  formatElapsed,
  formatEta,
  stageLabel,
  taskPhaseLabel,
  taskStatusLabel,
} from '../task-format';
import { ProcessingError } from './processing-error';

export interface TaskRowProps {
  task: BackgroundTask;
  onInspect: (task: BackgroundTask) => void;
}
export function TaskRow({ task, onInspect }: TaskRowProps) {
  const jobTask = task.kind === 'ai-processing' || task.kind === 'ocr';
  const recallTask =
    task.kind === 'recall-generation' ||
    task.kind === 'recall-indexing' ||
    task.kind === 'recall-index';
  const refresh = useBackgroundTaskStore((state) => state.refresh);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const failure = task.errorCode
    ? describeProcessingError(task.errorCode)
    : null;
  const handleInspect = useCallback(() => onInspect(task), [onInspect, task]);
  const perform = useCallback(
    async (
      action:
        | 'pause'
        | 'resume'
        | 'cancel'
        | 'retry'
        | 'retry-current'
        | 'reprocess-audio',
    ) => {
      if (pending) return;
      setPending(true);
      setError(null);
      try {
        if (
          recallTask &&
          (action === 'cancel' ||
            action === 'retry' ||
            action === 'pause' ||
            action === 'resume')
        ) {
          await controlRecallJob({ jobId: task.id, action });
        } else if (action === 'reprocess-audio' && task.recordingId) {
          await reprocessRecordingTranscription({
            recordingId: task.recordingId,
          });
        } else if (task.modelId) {
          if (action === 'cancel')
            await cancelAiModelDownload({ modelId: task.modelId });
          else if (action === 'retry' || action === 'retry-current')
            await downloadAiModel({ modelId: task.modelId });
        } else {
          const input = { jobId: task.id };
          switch (action) {
            case 'pause':
              await pauseProcessingJob(input);
              break;
            case 'resume':
              await resumeProcessingJob(input);
              break;
            case 'cancel':
              await cancelProcessingJob(input);
              break;
            case 'retry':
              await retryProcessingUnits(input);
              break;
            case 'retry-current':
              await retryProcessingUnits({
                ...input,
                useCurrentSettings: true,
              });
          }
        }
        await refresh();
      } catch (error) {
        setError(processingFailureSummary(error));
      } finally {
        setPending(false);
      }
    },
    [pending, recallTask, refresh, task],
  );
  const handlePause = useCallback(() => void perform('pause'), [perform]);
  const handleReprocessAudio = useCallback(
    () => void perform('reprocess-audio'),
    [perform],
  );
  const handleResume = useCallback(() => void perform('resume'), [perform]);
  const handleCancel = useCallback(() => void perform('cancel'), [perform]);
  const handleRetry = useCallback(() => void perform('retry'), [perform]);
  const handleRetryCurrent = useCallback(
    () => void perform('retry-current'),
    [perform],
  );

  return (
    <li className="p-6" aria-busy={pending}>
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-sm font-medium wrap-break-word">
              {task.title}
            </span>
            <span className="text-muted-foreground text-xs">
              {taskStatusLabel(task.status)}
            </span>
          </div>
          <div className="text-muted-foreground mt-1 flex flex-wrap gap-x-3 text-xs">
            <span>{taskPhaseLabel(task)}</span>
            <span>{formatElapsed(task.elapsedMs)}</span>
            <span>{formatEta(task)}</span>
            {task.progress !== null ? (
              <span className="tabular-nums">
                {Math.round(task.progress * 100)}%
              </span>
            ) : null}
          </div>
        </div>
        <div className="flex flex-wrap gap-1">
          {task.recordingId || recallTask ? (
            <Button onClick={handleInspect} size="sm" variant="ghost">
              Details
            </Button>
          ) : null}
          {(jobTask || recallTask) &&
          (task.status === 'running' || task.status === 'queued') ? (
            <Button
              disabled={pending}
              onClick={handlePause}
              size="sm"
              variant="outline"
            >
              Pause
            </Button>
          ) : null}
          {(jobTask || recallTask) && task.status === 'paused' ? (
            <Button
              disabled={pending}
              onClick={handleResume}
              size="sm"
              variant="outline"
            >
              Resume
            </Button>
          ) : null}
          {(jobTask || recallTask || task.modelId) &&
          (task.status === 'running' ||
            task.status === 'queued' ||
            task.status === 'paused') ? (
            <Button
              disabled={pending}
              onClick={handleCancel}
              size="sm"
              variant="ghost"
            >
              Cancel
            </Button>
          ) : null}
          {(jobTask || recallTask || task.modelId) &&
          (task.status === 'failed' ||
            task.status === 'completed-with-errors' ||
            task.status === 'cancelled') ? (
            <Button
              disabled={pending}
              onClick={handleRetry}
              size="sm"
              variant="outline"
            >
              Retry failed
            </Button>
          ) : null}
          {jobTask &&
          (task.status === 'failed' ||
            task.status === 'completed-with-errors' ||
            task.status === 'cancelled') ? (
            <Button
              disabled={pending}
              onClick={handleRetryCurrent}
              size="sm"
              variant="ghost"
            >
              Retry with current settings
            </Button>
          ) : null}
          {task.recordingId ? (
            <Button asChild size="sm" variant="ghost">
              <Link
                to="/library/$recordingId"
                params={{ recordingId: task.recordingId }}
              >
                Open recording
              </Link>
            </Button>
          ) : null}
          {task.dayId ? (
            <Button asChild size="sm" variant="ghost">
              <Link to="/day/$date" params={{ date: task.dayId }}>
                Open day
              </Link>
            </Button>
          ) : null}
          {task.kind === 'ai-processing' &&
          task.recordingId &&
          !['running', 'queued', 'paused'].includes(task.status) ? (
            <Button
              disabled={pending}
              onClick={handleReprocessAudio}
              size="sm"
              variant="ghost"
              title="Reprocess both audio tracks with current speech detection and transcription settings"
            >
              Reprocess audio tracks
            </Button>
          ) : null}
        </div>
      </div>
      <Progress
        className="mt-4 h-1.5"
        aria-label={`${task.title} progress`}
        value={task.progress === null ? null : task.progress * 100}
      />
      {task.stages.length ? (
        <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs">
          {task.stages.map((stage) => (
            <span key={stage.kind}>
              {stageLabel(stage.kind)}: {stage.completed + stage.failed}/
              {stage.total}
              {stage.failed ? ` · ${stage.failed} failed` : ''}
              {stage.skipped ? ` · ${stage.skipped} skipped` : ''}
            </span>
          ))}
        </div>
      ) : null}
      {failure && task.errorCode ? (
        <div className="mt-2">
          <ProcessingError errorCode={task.errorCode} />
        </div>
      ) : null}
      {error ? (
        <p className="text-destructive mt-2 text-xs" role="alert">
          {error}
        </p>
      ) : null}
    </li>
  );
}
