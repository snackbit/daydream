import { useCallback, useState } from 'react';
import type { RecordingProcessingLaneKind } from '@daydream/shared';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Button } from '@/components/ui/button';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { RecordingLaneDialog } from '@/features/processing/components/recording-lane-dialog';
import { RecordingProcessingCard } from '@/features/processing/components/recording-processing-card';
import { SharedProcessingWork } from '@/features/processing/components/shared-processing-work';
import { useRecordingProcessingStore } from '@/features/processing/recording-processing-store';
import { useRecordingProcessingSync } from '@/features/processing/use-recording-processing-sync';

export function ProcessingScreen() {
  useRecordingProcessingSync();
  const groups = useRecordingProcessingStore((state) => state.groups);
  const counts = useRecordingProcessingStore((state) => state.counts);
  const filter = useRecordingProcessingStore((state) => state.filter);
  const setFilter = useRecordingProcessingStore((state) => state.setFilter);
  const loading = useRecordingProcessingStore((state) => state.loading);
  const status = useRecordingProcessingStore((state) => state.status);
  const error = useRecordingProcessingStore((state) => state.error);
  const nextCursor = useRecordingProcessingStore((state) => state.nextCursor);
  const refresh = useRecordingProcessingStore((state) => state.refresh);
  const loadMore = useRecordingProcessingStore((state) => state.loadMore);
  const [selection, setSelection] = useState<{
    recordingId: string;
    kind: RecordingProcessingLaneKind;
  } | null>(null);
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
    <PageLayout>
      <PageHeader
        title="Processing"
        description="Follow each recording’s processing lanes and recover affected sections."
        actions={
          <Button
            size="sm"
            variant="outline"
            disabled={loading}
            onClick={reload}
          >
            Refresh
          </Button>
        }
      />
      <div className="text-muted-foreground flex flex-wrap gap-x-6 gap-y-1 text-sm">
        <span>{counts.active} recordings active or queued</span>
        <span>{counts.attention} recordings need attention</span>
      </div>
      <Tabs value={filter} onValueChange={changeFilter}>
        <TabsList className="h-auto max-w-full flex-wrap justify-start">
          <TabsTrigger value="all">All recordings</TabsTrigger>
          <TabsTrigger value="active">Active</TabsTrigger>
          <TabsTrigger value="attention">Needs attention</TabsTrigger>
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
        <ul aria-label="Recording processing" className="space-y-3">
          {groups.map((group) => (
            <RecordingProcessingCard
              key={group.recordingId}
              group={group}
              onInspect={inspect}
            />
          ))}
        </ul>
      ) : null}
      {nextCursor ? (
        <Button variant="outline" disabled={loading} onClick={loadEarlier}>
          Load earlier recordings
        </Button>
      ) : null}
      <SharedProcessingWork />
      <RecordingLaneDialog
        key={
          selection ? `${selection.recordingId}:${selection.kind}` : 'closed'
        }
        selection={selection}
        onClose={close}
      />
    </PageLayout>
  );
}
