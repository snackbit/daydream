import type { RecordingAiProcessingState } from '@daydream/shared';
import { formatTimelineOffset } from '../inspector/inspector-format';
import { operationLabels } from './reprocessing-dialog';

export interface ReprocessingStatusProps {
  state: RecordingAiProcessingState | null;
  queued: boolean;
}
export function ReprocessingStatus({ state, queued }: ReprocessingStatusProps) {
  const scopes = state?.reprocessingScopes ?? [],
    latest = scopes.at(-1)?.jobId;
  const current = scopes.filter((scope) => scope.jobId === latest);
  if (current.length === 0)
    return queued ? (
      <p role="status" className="text-muted-foreground text-sm">
        Reprocessing queued. Existing results remain available while their
        replacements run.
      </p>
    ) : null;
  const sources = new Map<string, typeof current>();
  for (const scope of current) {
    const key = `${scope.trackId}:${scope.operation}`;
    const group = sources.get(key) ?? [];
    group.push(scope);
    sources.set(key, group);
  }
  return (
    <div
      role="status"
      className="text-muted-foreground flex flex-wrap gap-x-6 gap-y-1 text-xs"
    >
      {[...sources].map(([key, ranges]) => {
        const first = ranges[0];
        if (!first) return null;
        const status = ranges.every((r) => r.status === 'succeeded')
          ? 'Complete'
          : ranges.some(
                (r) =>
                  r.status === 'failed' || r.status === 'completed-with-errors',
              )
            ? 'Needs attention'
            : ranges.some((r) => r.status === 'paused')
              ? 'Paused'
              : ranges.some(
                    (r) => r.status === 'queued' || r.status === 'running',
                  )
                ? 'Reprocessing'
                : 'Cancelled';
        return (
          <p key={key}>
            {first.sourceLabel} · {operationLabels[first.operation]} · {status}
            {ranges.length === 1
              ? ` · ${formatTimelineOffset(first.startOffsetMs)}–${formatTimelineOffset(first.endOffsetMs)}`
              : ` · ${ranges.length} ranges`}
          </p>
        );
      })}
    </div>
  );
}
