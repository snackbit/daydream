import type { RecorderError, RecorderStatus } from '@daydream/shared';
import type { RecorderLifecycleAction } from './recorder-workspace-store';

export function recorderLifecycleLocked(
  pendingLifecycleAction: RecorderLifecycleAction | null,
  status: RecorderStatus | null,
): boolean {
  return pendingLifecycleAction !== null || statusBlocksLifecycle(status);
}

export function recorderConfigurationLocked(
  pendingLifecycleAction: RecorderLifecycleAction | null,
  status: RecorderStatus | null,
): boolean {
  return (
    recorderLifecycleLocked(pendingLifecycleAction, status) ||
    status?.state === 'recording' ||
    status?.state === 'paused'
  );
}

export function statusBlocksLifecycle(status: RecorderStatus | null): boolean {
  return status?.finalization != null || statusShowsCommandInProgress(status);
}

export function statusShowsCommandInProgress(
  status: RecorderStatus | null,
): boolean {
  return (
    status?.state === 'pausing' ||
    status?.state === 'selecting' ||
    status?.state === 'stopping'
  );
}

export function isRecoverableRecorderActionError(error: unknown): boolean {
  return isRecorderError(error) && error.category === 'permission';
}

function isRecorderError(error: unknown): error is RecorderError {
  return (
    typeof error === 'object' &&
    error !== null &&
    'category' in error &&
    'code' in error &&
    'message' in error
  );
}
