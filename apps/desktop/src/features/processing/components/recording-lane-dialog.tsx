import { useCallback, useEffect, useRef, useState } from 'react';
import { Link } from '@tanstack/react-router';
import type {
  RecordingProcessingLaneDetail,
  RecordingProcessingLaneKind,
  RecordingProcessingUnit,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  getRecordingProcessingLane,
  listenToBackgroundTasksChanged,
} from '../processing-client';
import { processingFailureSummary } from '../processing-errors';
import { useRecordingProcessingStore } from '../recording-processing-store';
import { stageLabel } from '../task-format';
import { ProcessingLaneHistory } from './processing-lane-history';
import { TaskUnitRow } from './task-unit-row';

export interface RecordingLaneDialogProps {
  selection: { recordingId: string; kind: RecordingProcessingLaneKind } | null;
  onClose: () => void;
  onChanged?: () => Promise<void>;
}
export function RecordingLaneDialog({
  selection,
  onClose,
  onChanged,
}: RecordingLaneDialogProps) {
  const [detail, setDetail] = useState<RecordingProcessingLaneDetail | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(selection !== null);
  const [revision, setRevision] = useState(0);
  const requestVersion = useRef({ version: 0 });
  const loadedUnitCount = useRef(0);
  const globalRefresh = useRecordingProcessingStore((state) => state.refresh);
  const refresh = onChanged ?? globalRefresh;
  const recordingId = selection?.recordingId;
  const kind = selection?.kind;
  useEffect(() => {
    if (!recordingId || !kind) return;
    const owner = requestVersion.current;
    const ownedRequest = ++owner.version;
    const source = { recordingId, sourceKind: kind };
    async function readLoadedSections() {
      const target = loadedUnitCount.current;
      const input = {
        ...source,
        ...(target > 100 ? { limit: Math.min(200, target) } : {}),
      };
      let page = await getRecordingProcessingLane(input);
      let units = page.units;
      const cursors = new Set<string>();
      while (page.nextCursor && units.length < target) {
        if (owner.version !== ownedRequest) return null;
        const key = JSON.stringify(page.nextCursor);
        if (cursors.has(key))
          throw new Error('Processing sections could not be refreshed.');
        cursors.add(key);
        page = await getRecordingProcessingLane({
          ...input,
          cursor: page.nextCursor,
        });
        units = [...units, ...page.units];
      }
      return { ...page, units };
    }
    void readLoadedSections()
      .then((detail) => {
        if (owner.version === ownedRequest && detail) setDetail(detail);
      })
      .catch((error) => {
        if (owner.version === ownedRequest)
          setError(processingFailureSummary(error));
      })
      .finally(() => {
        if (owner.version === ownedRequest) setLoading(false);
      });
    return () => {
      if (owner.version === ownedRequest) owner.version++;
    };
  }, [recordingId, kind, revision]);
  useEffect(() => {
    if (!recordingId || !kind) return;
    let active = true,
      cleanup: (() => void) | undefined;
    const reconcile = () => {
      if (active) setRevision((value) => value + 1);
    };
    const timer = window.setInterval(reconcile, 5000);
    window.addEventListener('focus', reconcile);
    void listenToBackgroundTasksChanged((event) => {
      if (event.recordingId === recordingId) reconcile();
    })
      .then((unlisten) => {
        if (active) cleanup = unlisten;
        else unlisten();
      })
      .catch(reconcile);
    return () => {
      active = false;
      cleanup?.();
      window.clearInterval(timer);
      window.removeEventListener('focus', reconcile);
    };
  }, [recordingId, kind]);
  const onRetried = useCallback(async () => {
    await refresh();
    setLoading(true);
    setDetail(null);
    setError(null);
    setRevision((value) => value + 1);
  }, [refresh]);
  const retryRead = useCallback(() => {
    setLoading(true);
    setError(null);
    setRevision((value) => value + 1);
  }, []);
  const openChange = useCallback(
    (open: boolean) => {
      if (!open) onClose();
    },
    [onClose],
  );
  const loadMore = useCallback(async () => {
    if (!selection || !detail?.nextCursor || loading) return;
    setLoading(true);
    setError(null);
    const owner = requestVersion.current;
    const ownedRequest = owner.version;
    try {
      const page = await getRecordingProcessingLane({
        recordingId: selection.recordingId,
        sourceKind: selection.kind,
        cursor: detail.nextCursor,
      });
      if (owner.version === ownedRequest) {
        loadedUnitCount.current = detail.units.length + page.units.length;
        setDetail({ ...page, units: [...detail.units, ...page.units] });
      }
    } catch (error) {
      if (owner.version === ownedRequest)
        setError(processingFailureSummary(error));
    } finally {
      if (owner.version === ownedRequest) setLoading(false);
    }
  }, [detail, loading, selection]);
  const loadNext = useCallback(() => void loadMore(), [loadMore]);
  return (
    <Dialog open={selection !== null} onOpenChange={openChange}>
      <DialogContent className="max-h-[80dvh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>
            {selection
              ? `${stageLabel(selection.kind)} sections`
              : 'Processing sections'}
          </DialogTitle>
          <DialogDescription>
            Inspect source sections and retry affected work. Completed results
            remain available.
          </DialogDescription>
        </DialogHeader>
        {error ? (
          <div>
            <p
              role="alert"
              aria-label="Processing sections error"
              className="text-destructive text-sm"
            >
              {error}
            </p>
            {!detail ? (
              <Button variant="outline" onClick={retryRead}>
                Retry loading sections
              </Button>
            ) : null}
          </div>
        ) : null}
        {loading && !detail ? <p role="status">Loading sections…</p> : null}
        {detail && selection ? (
          <ul aria-label="Processing sections" className="divide-y">
            {detail.units.map((unit) => (
              <RecordingSection
                key={`${unit.jobId}:${unit.id}`}
                unit={unit}
                recordingId={selection.recordingId}
                onRetried={onRetried}
              />
            ))}
          </ul>
        ) : null}
        {detail?.nextCursor ? (
          <Button disabled={loading} onClick={loadNext} variant="outline">
            Load more sections
          </Button>
        ) : null}
        {selection ? (
          <ProcessingLaneHistory
            recordingId={selection.recordingId}
            kind={selection.kind}
          />
        ) : null}
        <Button variant="outline" onClick={onClose}>
          Close
        </Button>
      </DialogContent>
    </Dialog>
  );
}
interface RecordingSectionProps {
  unit: RecordingProcessingUnit;
  recordingId: string;
  onRetried: () => Promise<void>;
}
function RecordingSection({
  unit,
  recordingId,
  onRetried,
}: RecordingSectionProps) {
  return (
    <TaskUnitRow
      unit={unit}
      jobId={unit.jobId}
      onRetried={onRetried}
      sourceLink={
        unit.startOffsetMs !== null ? (
          <Button asChild variant="link" size="sm" className="px-0">
            <Link
              to="/library/$recordingId"
              params={{ recordingId }}
              search={{
                seek: unit.startOffsetMs,
                source: unit.sourceTrackId !== null ? unit.sourceRole : null,
                evidenceId: null,
              }}
            >
              Open source position
            </Link>
          </Button>
        ) : null
      }
    />
  );
}
