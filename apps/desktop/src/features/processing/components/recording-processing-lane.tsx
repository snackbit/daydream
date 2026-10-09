import { useCallback } from 'react';
import { Link } from '@tanstack/react-router';
import type {
  RecordingProcessingLane,
  RecordingProcessingLaneKind,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { stageLabel, taskStatusLabel } from '../task-format';
import { ProcessingError } from './processing-error';

export interface RecordingProcessingLaneProps {
  lane: RecordingProcessingLane;
  pending: boolean;
  onInspect: (kind: RecordingProcessingLaneKind) => void;
  onRetry: (
    kind: RecordingProcessingLaneKind,
    useCurrentSettings: boolean,
  ) => void;
}
function laneState(lane: RecordingProcessingLane) {
  switch (lane.applicability) {
    case 'disabled':
      return 'Disabled in settings';
    case 'no-source':
      return 'No source recorded';
    case 'protected':
      return 'Waiting for recording protection';
    case 'no-eligible-input':
      return 'No eligible source sections';
    case 'waiting':
      return 'Waiting for finalized recording segments';
    case 'available':
      if (
        lane.inputOpen &&
        lane.status !== 'paused' &&
        lane.completed + lane.skipped === lane.total &&
        !lane.failed &&
        !lane.cancelled
      )
        return 'Caught up · live recording';
      return lane.status ? taskStatusLabel(lane.status) : 'Not queued';
  }
}
export function RecordingProcessingLaneRow({
  lane,
  pending,
  onInspect,
  onRetry,
}: RecordingProcessingLaneProps) {
  const inspect = useCallback(
    () => onInspect(lane.kind),
    [lane.kind, onInspect],
  );
  const retry = useCallback(
    () => onRetry(lane.kind, false),
    [lane.kind, onRetry],
  );
  const retryCurrent = useCallback(
    () => onRetry(lane.kind, true),
    [lane.kind, onRetry],
  );
  const retryable =
    lane.applicability === 'available' &&
    (lane.failed > 0 || lane.cancelled > 0);
  return (
    <li className="min-w-0 py-3" aria-label={stageLabel(lane.kind)}>
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-sm font-medium">{stageLabel(lane.kind)}</p>
          <p className="text-muted-foreground mt-1 text-xs">
            {laneState(lane)}
          </p>
        </div>
        <div className="flex flex-wrap gap-1">
          {lane.jobIds.length ? (
            <Button size="sm" variant="ghost" onClick={inspect}>
              Details
            </Button>
          ) : null}
          {retryable ? (
            <>
              <Button
                size="sm"
                variant="outline"
                disabled={pending}
                onClick={retry}
              >
                Retry failed
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={pending}
                onClick={retryCurrent}
              >
                Use current settings
              </Button>
            </>
          ) : null}
        </div>
      </div>
      {lane.total ? (
        <>
          <Progress
            className="mt-2 h-1"
            aria-label={`${stageLabel(lane.kind)} sections attempted`}
            value={lane.progress === null ? null : lane.progress * 100}
          />
          <p className="text-muted-foreground mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs tabular-nums">
            <span>
              {lane.completed} complete / {lane.total} sections
            </span>
            {lane.failed ? (
              <span className="text-destructive">{lane.failed} failed</span>
            ) : null}
            {lane.skipped ? <span>{lane.skipped} skipped</span> : null}
            {lane.cancelled ? <span>{lane.cancelled} cancelled</span> : null}
            {lane.running ? <span>{lane.running} running</span> : null}
            {lane.queued ? <span>{lane.queued} queued</span> : null}
          </p>
        </>
      ) : null}
      {lane.nextRetryAtMs !== null ? (
        <p className="text-muted-foreground mt-1 text-xs">
          Automatic retry scheduled ·{' '}
          {new Date(lane.nextRetryAtMs).toLocaleTimeString([], {
            hour: '2-digit',
            minute: '2-digit',
          })}
        </p>
      ) : null}
      {lane.errorCode ? (
        <div className="mt-2">
          <ProcessingError errorCode={lane.errorCode} />
        </div>
      ) : null}
      {lane.applicability === 'protected' ? (
        <Button asChild size="sm" variant="link" className="mt-1 px-0">
          <Link to="/settings" search={{ tab: 'record', focus: null }}>
            Review recording protection
          </Link>
        </Button>
      ) : null}
    </li>
  );
}
