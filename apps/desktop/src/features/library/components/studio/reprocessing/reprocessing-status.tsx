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
  const sources = new Map<string, typeof current>();
  for (const scope of current) {
    const key = `${scope.trackId}:${scope.operation}`;
    const group = sources.get(key) ?? [];
    group.push(scope);
    sources.set(key, group);
  }
  const descriptions = [...sources.values()].flatMap((ranges) => {
    const first = ranges[0];
    if (!first) return [];
    const status = ranges.every((r) => r.status === 'succeeded')
      ? 'Complete'
      : ranges.some(
            (r) =>
              r.status === 'failed' || r.status === 'completed-with-errors',
          )
        ? 'Needs attention'
        : ranges.some((r) => r.status === 'paused')
          ? 'Paused'
          : ranges.some((r) => r.status === 'queued' || r.status === 'running')
            ? 'Reprocessing'
            : 'Cancelled';
    return [
      `${first.sourceLabel} · ${operationLabels[first.operation]} · ${status}${ranges.length === 1 ? ` · ${formatTimelineOffset(first.startOffsetMs)}–${formatTimelineOffset(first.endOffsetMs)}` : ` · ${ranges.length} ranges`}`,
    ];
  });
  const description = descriptions.length
    ? descriptions.join('  •  ')
    : queued
      ? 'Reprocessing queued. Fresh results will appear as processing finishes.'
      : '';
  return (
    <p
      role="status"
      className="text-muted-foreground h-4 shrink-0 truncate text-xs leading-4"
      title={description || undefined}
    >
      {description}
    </p>
  );
}
