import { useCallback, useEffect, useState } from 'react';
import type { BackgroundTask } from '@daydream/shared';
import { SharedDisclosure } from '@/components/shared-disclosure';
import { Button } from '@/components/ui/button';
import { DownloadTaskRow } from '@/features/ai/models/download-task-row';
import { useBackgroundTaskStore } from '../background-task-store';
import { TaskDetailDialog } from './task-detail-dialog';
import { TaskRow } from './task-row';

export function SharedProcessingWork() {
  const tasks = useBackgroundTaskStore((state) => state.tasks);
  const refresh = useBackgroundTaskStore((state) => state.refresh);
  const error = useBackgroundTaskStore((state) => state.error);
  const loading = useBackgroundTaskStore((state) => state.loading);
  const nextCursor = useBackgroundTaskStore((state) => state.nextCursor);
  const loadMore = useBackgroundTaskStore((state) => state.loadMore);
  const [selected, setSelected] = useState<BackgroundTask | null>(null);
  useEffect(() => {
    void refresh();
  }, [refresh]);
  const inspect = useCallback((task: BackgroundTask) => setSelected(task), []);
  const close = useCallback(() => setSelected(null), []);
  const reload = useCallback(() => void refresh(), [refresh]);
  const loadEarlier = useCallback(() => void loadMore(), [loadMore]);
  const shared = tasks.filter(
    (task) =>
      task.recordingId === null &&
      !['succeeded', 'cancelled'].includes(task.status),
  );
  return (
    <>
      <SharedDisclosure
        label="Shared work"
        status={
          shared.length
            ? `${shared.length} active or needing attention`
            : 'No active shared work'
        }
      >
        <p className="text-muted-foreground mb-2 text-xs">
          Model downloads and Recall jobs support several recordings. Their
          controls affect shared work.
        </p>
        {error ? (
          <p role="alert" className="text-destructive text-sm">
            {error}
          </p>
        ) : null}
        {shared.length ? (
          <ul aria-label="Shared background tasks" className="divide-y">
            {shared.map((task) =>
              task.modelId ? (
                <DownloadTaskRow
                  key={task.id}
                  task={task}
                  onChanged={refresh}
                />
              ) : (
                <TaskRow key={task.id} task={task} onInspect={inspect} />
              ),
            )}
          </ul>
        ) : null}
        {nextCursor ? (
          <Button
            size="sm"
            variant="outline"
            disabled={loading}
            onClick={loadEarlier}
          >
            Load earlier shared work
          </Button>
        ) : null}
        <Button size="sm" variant="ghost" disabled={loading} onClick={reload}>
          Refresh shared work
        </Button>
      </SharedDisclosure>
      <TaskDetailDialog
        key={selected?.id ?? 'closed'}
        task={
          selected
            ? (tasks.find((task) => task.id === selected.id) ?? selected)
            : null
        }
        onClose={close}
      />
    </>
  );
}
