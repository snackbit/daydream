import { useCallback, useState } from 'react';
import type {
  RecordingProcessingLaneDetail,
  RecordingProcessingLaneKind,
} from '@daydream/shared';
import { SharedDisclosure } from '@/components/shared-disclosure';
import { Button } from '@/components/ui/button';
import { getRecordingProcessingLane } from '../processing-client';
import { processingFailureSummary } from '../processing-errors';
import { formatElapsed, taskStatusLabel } from '../task-format';

export interface ProcessingLaneHistoryProps {
  recordingId: string;
  kind: RecordingProcessingLaneKind;
}
export function ProcessingLaneHistory({
  recordingId,
  kind,
}: ProcessingLaneHistoryProps) {
  const [detail, setDetail] = useState<RecordingProcessingLaneDetail | null>(
    null,
  );
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(async () => {
    if (pending) return;
    setPending(true);
    setError(null);
    try {
      const page = await getRecordingProcessingLane({
        recordingId,
        sourceKind: kind,
        history: true,
        cursor: detail?.nextCursor ?? null,
      });
      setDetail({ ...page, units: [...(detail?.units ?? []), ...page.units] });
    } catch (error) {
      setError(processingFailureSummary(error));
    } finally {
      setPending(false);
    }
  }, [detail, kind, pending, recordingId]);
  const loadHistory = useCallback(() => void load(), [load]);
  return (
    <SharedDisclosure
      label="Earlier runs"
      status={
        detail
          ? `${detail.units.length} earlier sections`
          : 'Excluded from current progress'
      }
    >
      <p className="text-muted-foreground mb-2 text-xs">
        Earlier runs remain inspectable. Retry current work from the recording’s
        lane controls.
      </p>
      {detail?.units.length ? (
        <ul aria-label="Earlier processing sections" className="divide-y">
          {detail.units.map((unit) => (
            <li key={`${unit.jobId}:${unit.id}`} className="py-2 text-xs">
              {unit.startOffsetMs === null
                ? 'Whole source'
                : formatElapsed(unit.startOffsetMs)}{' '}
              · {taskStatusLabel(unit.status)} · attempt {unit.attemptCount}
            </li>
          ))}
        </ul>
      ) : detail ? (
        <p className="text-muted-foreground text-xs">No earlier runs.</p>
      ) : null}
      {error ? (
        <p role="alert" className="text-destructive text-xs">
          {error}
        </p>
      ) : null}
      {!detail || detail.nextCursor ? (
        <Button
          disabled={pending}
          variant="outline"
          size="sm"
          onClick={loadHistory}
        >
          {detail ? 'Load more earlier sections' : 'Load earlier sections'}
        </Button>
      ) : null}
    </SharedDisclosure>
  );
}
