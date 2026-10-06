import { useCallback, useEffect, useState } from 'react';
import type { BackgroundTask } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useBackgroundTaskStore } from '@/features/processing/background-task-store';
import { TaskDetailDialog } from '@/features/processing/components/task-detail-dialog';
import { TaskRow } from '@/features/processing/components/task-row';
import { taskNeedsAttention } from '@/features/processing/task-format';

type Filter = 'all' | 'active' | 'attention' | 'history';
export function ProcessingScreen() {
  const tasks = useBackgroundTaskStore((state) => state.tasks);
  const status = useBackgroundTaskStore((state) => state.status);
  const error = useBackgroundTaskStore((state) => state.error);
  const loading = useBackgroundTaskStore((state) => state.loading);
  const refresh = useBackgroundTaskStore((state) => state.refresh);
  const loadMore = useBackgroundTaskStore((state) => state.loadMore);
  const nextCursor = useBackgroundTaskStore((state) => state.nextCursor);
  const [filter, setFilter] = useState<Filter>('all');
  const [selected, setSelected] = useState<BackgroundTask | null>(null);
  const selectedTask = selected
    ? (tasks.find((task) => task.id === selected.id) ?? selected)
    : null;
  useEffect(() => {
    if (status === 'idle') void refresh();
  }, [refresh, status]);
  const handleRefresh = useCallback(() => void refresh(), [refresh]);
  const handleLoadMore = useCallback(() => void loadMore(), [loadMore]);
  const handleInspect = useCallback(
    (task: BackgroundTask) => setSelected(task),
    [],
  );
  const handleClose = useCallback(() => setSelected(null), []);
  const handleFilterChange = useCallback((value: string) => {
    if (
      value === 'all' ||
      value === 'active' ||
      value === 'attention' ||
      value === 'history'
    )
      setFilter(value);
  }, []);
  const active = tasks.filter(
    (task) => task.status === 'running' || task.status === 'queued',
  ).length;
  const failed = tasks.filter(taskNeedsAttention).length;
  const visible = tasks.filter(
    (task) =>
      filter === 'all' ||
      (filter === 'active' &&
        ['running', 'queued', 'paused'].includes(task.status)) ||
      (filter === 'attention' && taskNeedsAttention(task)) ||
      (filter === 'history' &&
        ['succeeded', 'cancelled', 'completed-with-errors', 'failed'].includes(
          task.status,
        )),
  );
  return (
    <section className="mx-auto flex max-w-6xl flex-col gap-4 p-4">
      <header className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-lg font-semibold">Processing</h1>
          <p className="text-muted-foreground mt-1 text-xs">
            Background work stays independent of recording and navigation.
          </p>
        </div>
        <Button
          disabled={loading}
          onClick={handleRefresh}
          size="sm"
          variant="outline"
        >
          Refresh
        </Button>
      </header>
      <div className="flex flex-wrap gap-x-6 gap-y-2 border-y py-3 text-sm">
        <span>{active} active or queued</span>
        <span>{failed} need attention</span>
        <span className="text-muted-foreground">
          Completed results remain available
        </span>
      </div>
      <Tabs value={filter} onValueChange={handleFilterChange}>
        <TabsList className="rounded-none">
          <TabsTrigger value="all">All work</TabsTrigger>
          <TabsTrigger value="active">Active</TabsTrigger>
          <TabsTrigger value="attention">Needs attention</TabsTrigger>
          <TabsTrigger value="history">History</TabsTrigger>
        </TabsList>
      </Tabs>
      {error ? (
        <div role="alert" className="text-destructive border p-3 text-sm">
          {error}
        </div>
      ) : null}
      {status === 'loading' && !tasks.length ? (
        <p role="status" className="text-muted-foreground text-sm">
          Loading processing activity…
        </p>
      ) : null}
      {status === 'ready' && !visible.length ? (
        <p className="text-muted-foreground border p-4 text-sm">
          No processing work in this view.
        </p>
      ) : null}
      {visible.length ? (
        <ul aria-label="Background tasks" className="border">
          {visible.map((task) => (
            <TaskRow key={task.id} task={task} onInspect={handleInspect} />
          ))}
        </ul>
      ) : null}
      {nextCursor ? (
        <Button disabled={loading} onClick={handleLoadMore} variant="outline">
          Load earlier work
        </Button>
      ) : null}
      <TaskDetailDialog
        key={selected?.id ?? 'closed'}
        task={selectedTask}
        onClose={handleClose}
      />
    </section>
  );
}
