import type { StudioTimelineBucket } from '@daydream/shared';

export function decodeStudioOverview(value: unknown): StudioTimelineBucket[] {
  if (!Array.isArray(value)) throw new Error('Invalid Studio overview.');
  return value.map((item: unknown) => {
    if (!item || typeof item !== 'object')
      throw new Error('Invalid Studio overview.');
    const row: Record<string, unknown> = Object.fromEntries(
      Object.entries(item),
    );
    const { trackId, kind, startOffsetMs, endOffsetMs, count } = row;
    if (
      typeof trackId !== 'string' ||
      !trackId ||
      typeof startOffsetMs !== 'number' ||
      !Number.isSafeInteger(startOffsetMs) ||
      startOffsetMs < 0 ||
      typeof endOffsetMs !== 'number' ||
      !Number.isSafeInteger(endOffsetMs) ||
      endOffsetMs <= startOffsetMs ||
      typeof count !== 'number' ||
      !Number.isSafeInteger(count) ||
      count <= 0 ||
      (kind !== 'speech' && kind !== 'observation' && kind !== 'activity')
    )
      throw new Error('Invalid Studio overview.');
    return { trackId, kind, startOffsetMs, endOffsetMs, count };
  });
}
