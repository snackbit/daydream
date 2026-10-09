import type { RecorderStatus } from '@daydream/shared';

type StatusOrder = Pick<RecorderStatus, 'statusRevision' | 'updatedAtMs'>;
export function recorderStatusIsCurrent(
  current: StatusOrder | null,
  incoming: StatusOrder,
): boolean {
  if (
    current?.statusRevision !== undefined &&
    incoming.statusRevision !== undefined
  ) {
    return incoming.statusRevision >= current.statusRevision;
  }
  return !current || incoming.updatedAtMs >= current.updatedAtMs;
}
