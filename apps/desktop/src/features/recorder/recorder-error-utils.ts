import type { RecorderError } from '@daydream/shared';

export function errorMessage(error: unknown): string {
  if (isRecorderError(error)) {
    return error.message;
  }

  return error instanceof Error ? error.message : String(error);
}

export function isRecorderPortalSelectionCancelled(error: unknown): boolean {
  return (
    isRecorderError(error) && error.code === 'recorder_portal_permission_denied'
  );
}

export function isRecorderError(error: unknown): error is RecorderError {
  return (
    typeof error === 'object' &&
    error !== null &&
    'category' in error &&
    'code' in error &&
    'message' in error
  );
}
