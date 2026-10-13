import type { ProcessingLaneActivity } from '@daydream/shared';
import { formatDuration } from '@/features/library/library-format';
import { processingActivityLabel } from '../processing-activity';
import { formatElapsed } from '../task-format';

export interface ProcessingActivityListProps {
  activity: readonly ProcessingLaneActivity[];
  now: number;
}
export function ProcessingActivityList({
  activity,
  now,
}: ProcessingActivityListProps) {
  if (!activity.length)
    return (
      <p className="text-muted-foreground text-xs">
        Current source details are unavailable.
      </p>
    );
  return (
    <ul aria-label="Current and next work" className="divide-y">
      {activity.map((item) => (
        <li key={item.chunkId} className="py-2 text-xs">
          <p className="font-medium">{processingActivityLabel(item)}</p>
          {item.sourceLabel || item.startOffsetMs !== null ? (
            <p className="text-muted-foreground mt-1">
              {item.sourceLabel ?? 'Source section'}
              {item.startOffsetMs !== null
                ? ` · ${formatDuration(item.startOffsetMs)}${item.endOffsetMs !== null ? `–${formatDuration(item.endOffsetMs)}` : ''}`
                : ''}
            </p>
          ) : null}
          <p className="text-muted-foreground mt-1 flex flex-wrap gap-x-3 gap-y-1 tabular-nums">
            {item.status === 'running' && item.phaseStartedAtMs !== null ? (
              <span>
                This phase: {formatElapsed(now - item.phaseStartedAtMs)}
              </span>
            ) : null}
            {item.status === 'running' ? (
              <span>
                {item.lastProgressAtMs === null
                  ? 'No measured progress yet'
                  : `Last progress: ${formatElapsed(now - item.lastProgressAtMs)} ago`}
              </span>
            ) : null}
            {item.nextRetryAtMs !== null && item.waitingReason === 'retry' ? (
              <span>
                Retry at {new Date(item.nextRetryAtMs).toLocaleTimeString()}
              </span>
            ) : null}
          </p>
        </li>
      ))}
    </ul>
  );
}
