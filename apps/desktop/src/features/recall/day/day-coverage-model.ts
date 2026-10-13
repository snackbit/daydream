import type { RecallCoverageInterval } from '@daydream/shared';

export interface DayCoverageSource {
  recordingId: string;
  kind: RecallCoverageInterval['kind'];
  sourceTrackIds: string[];
}
export interface DayCoverageSpan {
  startMs: number;
  endMs: number;
  sources: DayCoverageSource[];
}

/** Segment IDs are provenance, not display boundaries. Scope and status are. */
export function buildDayCoverageSpans(
  intervals: RecallCoverageInterval[],
): DayCoverageSpan[] {
  const events = new Map<
    number,
    { key: string; delta: number; source: DayCoverageSource }[]
  >();
  for (const interval of intervals) {
    if (interval.endMs <= interval.startMs) continue;
    const source: DayCoverageSource = {
      recordingId: interval.recordingId,
      kind: interval.kind,
      sourceTrackIds: [...new Set(interval.sourceTrackIds)].sort(),
    };
    const key = JSON.stringify(source);
    for (const [time, delta] of [
      [interval.startMs, 1],
      [interval.endMs, -1],
    ] as const) {
      const changes = events.get(time) ?? [];
      changes.push({ key, delta, source });
      events.set(time, changes);
    }
  }
  const active = new Map<
    string,
    { count: number; source: DayCoverageSource }
  >();
  const spans: DayCoverageSpan[] = [];
  let previousTime: number | undefined;
  let previousSignature = '';
  for (const [time, changes] of [...events.entries()].sort(
    ([a], [b]) => a - b,
  )) {
    if (previousTime !== undefined && active.size > 0) {
      const sources = [...active.entries()].sort(([a], [b]) =>
        a.localeCompare(b),
      );
      const signature = JSON.stringify(sources.map(([key]) => key));
      const previous = spans.at(-1);
      if (previous?.endMs === previousTime && signature === previousSignature) {
        spans[spans.length - 1] = { ...previous, endMs: time };
      } else {
        spans.push({
          startMs: previousTime,
          endMs: time,
          sources: sources.map(([, value]) => value.source),
        });
      }
      previousSignature = signature;
    }
    for (const { key, delta, source } of changes) {
      const count = (active.get(key)?.count ?? 0) + delta;
      if (count === 0) active.delete(key);
      else active.set(key, { count, source });
    }
    previousTime = time;
  }
  return spans;
}

export function coverageSourceLabel(source: DayCoverageSource): string {
  switch (source.kind) {
    case 'captured':
      return 'Captured';
    case 'uninterpreted':
      return 'Not interpreted yet';
    case 'missing':
      return 'Missing source · timing unavailable';
    case 'gap':
      return 'No recording · activity unknown';
    case 'redacted':
      return source.sourceTrackIds.length === 0
        ? 'Protected · all sources excluded'
        : `Protected · selected sources excluded (${source.sourceTrackIds.join(', ')})`;
  }
}

export function coverageSpanKind(
  span: DayCoverageSpan,
): RecallCoverageInterval['kind'] {
  for (const kind of [
    'missing',
    'uninterpreted',
    'captured',
    'redacted',
    'gap',
  ] as const)
    if (span.sources.some((source) => source.kind === kind)) return kind;
  return 'gap';
}
