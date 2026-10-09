import type { RecorderScene, RecorderStatus } from '@daydream/shared';

export interface CaptureStartBlockerOptions {
  isDirty: boolean;
  lifecycleLocked: boolean;
  loadStatus: string;
  scene: RecorderScene | null;
  status: RecorderStatus | null;
}

export function getCaptureStartBlocker({
  lifecycleLocked,
  loadStatus,
  scene,
  status,
}: CaptureStartBlockerOptions): string | null {
  if (lifecycleLocked) {
    return 'Wait for the current capture action to finish.';
  }
  if (loadStatus === 'error') {
    return 'Capture setup could not load. Refresh the workspace and try again.';
  }
  if (loadStatus !== 'ready' || !status) {
    return 'Checking capture readiness…';
  }
  if (status.state !== 'idle') {
    return status.lastError?.message ?? 'Capture is not ready to start.';
  }
  if (!scene || scene.sources.length === 0) {
    return 'Add at least one monitor source before recording.';
  }
  if (!status.backendHealth.portal.available) {
    return (
      status.backendHealth.portal.error ??
      'Screen capture permission is unavailable.'
    );
  }
  if (!status.backendHealth.pipewire.available) {
    return (
      status.backendHealth.pipewire.error ?? 'Desktop capture is unavailable.'
    );
  }
  if (!status.backendHealth.mediaWriter.available) {
    return (
      status.backendHealth.mediaWriter.error ??
      'The local media writer is unavailable.'
    );
  }

  return null;
}
