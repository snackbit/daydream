import type { RecorderPreviewStatus } from '@daydream/shared';
import { isRecorderError } from './recorder-error-utils';
import type {
  RecorderWorkspaceGet,
  RecorderWorkspaceSet,
} from './recorder-workspace-store-types';

export function previewIsActive(status: RecorderPreviewStatus | null): boolean {
  return status?.state === 'starting' || status?.state === 'live';
}

export async function stopPreviewForRecording(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
): Promise<boolean> {
  await get().stopPreview();
  if (get().previewStatus?.state !== 'error') return true;
  set({ error: get().error ?? 'Preview could not stop. Try again.' });
  return false;
}

export function previewErrorStatus(
  sceneId: string | null | undefined,
  error: unknown,
): RecorderPreviewStatus {
  return {
    error: isRecorderError(error) ? error : null,
    sceneId: sceneId ?? null,
    sourceIds: [],
    state: 'error',
    updatedAtMs: Date.now(),
  };
}
