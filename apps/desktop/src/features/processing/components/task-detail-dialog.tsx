import { useCallback, useEffect, useState } from 'react';
import type { BackgroundTask, BackgroundTaskDetail } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { useBackgroundTaskStore } from '../background-task-store';
import { getBackgroundTask } from '../processing-client';
import { processingFailureSummary } from '../processing-errors';
import { taskPhaseLabel } from '../task-format';
import { ProcessingError } from './processing-error';
import { TaskUnitRow } from './task-unit-row';

export interface TaskDetailDialogProps {
  task: BackgroundTask | null;
  onClose: () => void;
}
export function TaskDetailDialog({ task, onClose }: TaskDetailDialogProps) {
  const [detail, setDetail] = useState<BackgroundTaskDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const refresh = useBackgroundTaskStore((state) => state.refresh);
  useEffect(() => {
    if (!task) return;
    let active = true;
    void getBackgroundTask({ taskId: task.id })
      .then((value) => {
        if (active) {
          setDetail(value);
          setError(null);
        }
      })
      .catch((error) => {
        if (active) {
          setError(processingFailureSummary(error));
          setDetail(null);
        }
      });
    return () => {
      active = false;
    };
  }, [task]);
  const handleOpenChange = useCallback(
    (open: boolean) => {
      if (!open) onClose();
    },
    [onClose],
  );
  return (
    <Dialog open={task !== null} onOpenChange={handleOpenChange}>
      <DialogContent className="max-h-[80dvh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{task?.title ?? 'Processing details'}</DialogTitle>
          <DialogDescription>
            Source sections, progress, attempts, and recovery details.
          </DialogDescription>
        </DialogHeader>
        {error ? (
          <p role="alert" className="text-destructive text-sm">
            {error}
          </p>
        ) : null}
        {!detail && !error ? (
          <p role="status" className="text-muted-foreground text-sm">
            Loading task details…
          </p>
        ) : null}
        {detail ? (
          <ul aria-label="Processing sections" className="divide-y">
            {detail.units.map((unit) => (
              <TaskUnitRow
                key={unit.id}
                unit={unit}
                jobId={detail.task.id}
                onRetried={refresh}
              />
            ))}
          </ul>
        ) : null}
        {detail && !detail.units.length ? (
          <div className="space-y-2">
            <p className="text-muted-foreground text-sm">
              {taskPhaseLabel(detail.task)} · Shared work has no recording
              sections.
            </p>
            {detail.task.errorCode ? (
              <ProcessingError errorCode={detail.task.errorCode} />
            ) : null}
          </div>
        ) : null}
        <Button onClick={onClose} variant="outline">
          Close
        </Button>
      </DialogContent>
    </Dialog>
  );
}
