import type {
  RecorderPreviewFrame,
  RecorderPreviewStatus,
} from '@daydream/shared';
import { stopRecorderPreview } from './recorder-client';
import { errorMessage, isRecorderError } from './recorder-error-utils';

type PreviewMutation = {
  error?: string | null;
  previewFramesBySceneSourceId?: Record<string, RecorderPreviewFrame>;
  previewStatus?: RecorderPreviewStatus | null;
};

export function previewIsActive(status: RecorderPreviewStatus | null): boolean {
  return status?.state === 'starting' || status?.state === 'live';
}

export async function stopPreviewForRecording(
  set: (partial: PreviewMutation) => void,
  sceneId: string | null,
): Promise<boolean> {
  try {
    const previewStatus = await stopRecorderPreview();
    set({
      previewFramesBySceneSourceId: {},
      previewStatus,
    });

    return true;
  } catch (error) {
    set({
      error: errorMessage(error),
      previewStatus: previewErrorStatus(sceneId, error),
    });

    return false;
  }
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
