import { useState } from 'react';
import type { BackgroundTaskUnit } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { retryProcessingUnits } from '../processing-client';
import { processingFailureSummary } from '../processing-errors';
import { formatElapsed, stageLabel, taskStatusLabel } from '../task-format';
import { ProcessingError } from './processing-error';

export interface TaskUnitRowProps {
  unit: BackgroundTaskUnit;
  jobId: string;
  onRetried: () => Promise<void>;
}
export function TaskUnitRow({ unit, jobId, onRetried }: TaskUnitRowProps) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function retry(useCurrentSettings: boolean) {
    setBusy(true);
    setError(null);
    try {
      await retryProcessingUnits({
        jobId,
        chunkId: unit.chunkId,
        unitId: unit.id,
        useCurrentSettings,
      });
      await onRetried();
    } catch (error) {
      setError(processingFailureSummary(error));
    } finally {
      setBusy(false);
    }
  }
  function retrySaved() {
    void retry(false);
  }
  function retryCurrent() {
    void retry(true);
  }
  const failed =
    unit.status === 'failed' ||
    unit.status === 'completed-with-errors' ||
    unit.status === 'cancelled';
  return (
    <li className="py-2 text-xs">
      <div className="flex flex-wrap justify-between gap-2">
        <span>
          {stageLabel(unit.sourceKind)} ·{' '}
          {unit.startOffsetMs === null
            ? 'Whole source'
            : formatElapsed(unit.startOffsetMs)}
          {unit.endOffsetMs !== null
            ? `–${formatElapsed(unit.endOffsetMs)}`
            : ''}
        </span>
        <span>
          {taskStatusLabel(unit.status)} · attempt {unit.attemptCount}
        </span>
      </div>
      {unit.status === 'running' ? (
        <Progress
          aria-label="Section progress"
          className="my-1 h-1"
          value={unit.progress * 100}
        />
      ) : null}
      {unit.errorCode ? <ProcessingError errorCode={unit.errorCode} /> : null}
      {failed ? (
        <div className="mt-1 flex gap-1">
          <Button
            variant="outline"
            size="sm"
            disabled={busy}
            onClick={retrySaved}
          >
            Retry section
          </Button>
          <Button
            variant="ghost"
            size="sm"
            disabled={busy}
            onClick={retryCurrent}
          >
            Use current settings
          </Button>
        </div>
      ) : null}
      {error ? (
        <p role="alert" className="text-destructive">
          {error}
        </p>
      ) : null}
    </li>
  );
}
