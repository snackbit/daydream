import { useCallback, useEffect, useState } from 'react';
import type { RecordingProcessingLaneKind } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  useRecordingProcessingStore,
  type RecordingProcessingStore,
} from '../recording-processing-store';
import { useRecordingProcessingSync } from '../use-recording-processing-sync';
import { RecordingLaneDialog } from './recording-lane-dialog';
import { RecordingProcessingCard } from './recording-processing-card';

export interface ProcessingOverviewProps {
  store?: RecordingProcessingStore;
  active?: boolean;
}
export function ProcessingOverview({
  store: useStore = useRecordingProcessingStore,
  active = true,
}: ProcessingOverviewProps) {
  useRecordingProcessingSync(useStore, active);
  const groups = useStore((state) => state.groups),
    counts = useStore((state) => state.counts);
  const filter = useStore((state) => state.filter),
    setFilter = useStore((state) => state.setFilter);
  const loading = useStore((state) => state.loading),
    status = useStore((state) => state.status),
    error = useStore((state) => state.error);
  const nextCursor = useStore((state) => state.nextCursor),
    refresh = useStore((state) => state.refresh),
    loadMore = useStore((state) => state.loadMore);
  const [selection, setSelection] = useState<{
    recordingId: string;
    kind: RecordingProcessingLaneKind;
  } | null>(null);
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    if (!active || !counts.active) return;
    const timer = window.setInterval(() => setNow(Date.now()), 5000);
    return () => window.clearInterval(timer);
  }, [active, counts.active]);
  const inspect = useCallback(
    (recordingId: string, kind: RecordingProcessingLaneKind) =>
      setSelection({ recordingId, kind }),
    [],
  );
  const close = useCallback(() => setSelection(null), []);
  const reload = useCallback(() => void refresh(), [refresh]);
  const loadEarlier = useCallback(() => void loadMore(), [loadMore]);
  const changeFilter = useCallback(
    (value: string) => {
      if (
        value === 'all' ||
        value === 'active' ||
        value === 'attention' ||
        value === 'history'
      )
        setFilter(value);
    },
    [setFilter],
  );
  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div
          aria-label="Processing totals"
          className="flex flex-wrap gap-x-6 gap-y-2 text-sm tabular-nums"
        >
          {counts.running !== undefined ? (
            <span>
              <strong>{counts.running}</strong> running
            </span>
          ) : null}
          {counts.queued !== undefined ? (
            <span>
              <strong>{counts.queued}</strong> queued
            </span>
          ) : null}
          <span>
            <strong>{counts.attention}</strong> recordings need attention
          </span>
          <span className="text-muted-foreground">
            {counts.active} recordings active or queued
          </span>
        </div>
        <Button size="sm" variant="outline" disabled={loading} onClick={reload}>
          Refresh
        </Button>
      </div>
      <Tabs value={filter} onValueChange={changeFilter}>
        <TabsList className="h-auto max-w-full flex-wrap justify-start">
          <TabsTrigger value="active">Active</TabsTrigger>
          <TabsTrigger value="attention">Needs attention</TabsTrigger>
          <TabsTrigger value="all">All recordings</TabsTrigger>
          <TabsTrigger value="history">History</TabsTrigger>
        </TabsList>
      </Tabs>
      {error ? (
        <p role="alert" className="text-destructive text-sm">
          {error}
        </p>
      ) : null}
      {status === 'loading' && !groups.length ? (
        <p role="status" className="text-muted-foreground text-sm">
          Loading recording activity…
        </p>
      ) : null}
      {status === 'ready' && !groups.length ? (
        <p className="text-muted-foreground bg-card rounded-panel p-4 text-sm">
          No recordings in this view.
        </p>
      ) : null}
      {groups.length ? (
        <ul aria-label="Recording processing" className="space-y-4">
          {groups.map((group) => (
            <RecordingProcessingCard
              key={group.recordingId}
              group={group}
              onInspect={inspect}
              onChanged={refresh}
              now={now}
            />
          ))}
        </ul>
      ) : null}
      {nextCursor ? (
        <Button variant="outline" disabled={loading} onClick={loadEarlier}>
          Load earlier recordings
        </Button>
      ) : null}
      <RecordingLaneDialog
        key={
          selection ? `${selection.recordingId}:${selection.kind}` : 'closed'
        }
        selection={selection}
        onClose={close}
        onChanged={refresh}
      />
    </div>
  );
}
