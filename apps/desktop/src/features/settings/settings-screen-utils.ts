import type { AppSettings, AppSettingsLimits } from '@daydream/shared';
import { unknownErrorMessage } from '@/lib/async-store-action';

export function validateDraftSettings(
  draft: AppSettings,
  limits: AppSettingsLimits,
): string | null {
  if (
    !Number.isFinite(draft.record.fps) ||
    draft.record.fps < limits.record.fpsMin ||
    draft.record.fps > limits.record.fpsMax
  ) {
    return 'Frame cadence must be between one frame every 60 seconds and 5 fps.';
  }
  if (
    draft.record.videoBitrateKbps < limits.record.videoBitrateKbpsMin ||
    draft.record.videoBitrateKbps > limits.record.videoBitrateKbpsMax
  ) {
    return 'Video bitrate must be between 250 and 10000 kbps.';
  }
  if (
    draft.storage.retentionDays < limits.storage.retentionDaysMin ||
    draft.storage.retentionDays > limits.storage.retentionDaysMax
  ) {
    return 'Retention must be between 1 and 365 days.';
  }
  if (draft.storage.sizeLimitBytes < limits.storage.sizeLimitBytesMin) {
    return 'Storage size limit must be at least 1 GiB.';
  }
  if (
    draft.ai.processing.maxParallelJobs <
      limits.ai.processing.maxParallelJobsMin ||
    draft.ai.processing.maxParallelJobs >
      limits.ai.processing.maxParallelJobsMax
  ) {
    return 'Max parallel AI jobs must be between 1 and 4.';
  }
  if (
    draft.ai.processing.maxParallelChunksPerJob <
      limits.ai.processing.maxParallelChunksPerJobMin ||
    draft.ai.processing.maxParallelChunksPerJob >
      limits.ai.processing.maxParallelChunksPerJobMax
  ) {
    return 'Max parallel AI chunks must be between 1 and 8.';
  }
  if (
    !Number.isFinite(draft.ai.processing.frameRecognitionFps) ||
    draft.ai.processing.frameRecognitionFps <
      limits.ai.processing.frameRecognitionFpsMin ||
    draft.ai.processing.frameRecognitionFps >
      limits.ai.processing.frameRecognitionFpsMax
  ) {
    return 'Frame recognition cadence must be between one frame every 60 seconds and 5 fps.';
  }
  if (
    draft.ai.processing.periodicFullFrameIntervalMs <
      limits.ai.processing.periodicFullFrameIntervalMsMin ||
    draft.ai.processing.periodicFullFrameIntervalMs >
      limits.ai.processing.periodicFullFrameIntervalMsMax
  ) {
    return 'Full-frame interval must be between 5 seconds and 10 minutes.';
  }
  if (
    draft.ai.processing.maxFullFrameCheckpointsPerMinute <
      limits.ai.processing.maxFullFrameCheckpointsPerMinuteMin ||
    draft.ai.processing.maxFullFrameCheckpointsPerMinute >
      limits.ai.processing.maxFullFrameCheckpointsPerMinuteMax
  ) {
    return 'Max full-frame checkpoints must be between 1 and 60 per minute.';
  }
  if (
    draft.ai.processing.visualDeltaTileSizePx <
      limits.ai.processing.visualDeltaTileSizePxMin ||
    draft.ai.processing.visualDeltaTileSizePx >
      limits.ai.processing.visualDeltaTileSizePxMax
  ) {
    return 'Visual delta tile size must be between 32 and 256 px.';
  }
  if (
    draft.ai.activity.keyboard.windowMs <
      limits.ai.activity.keyboardWindowMsMin ||
    draft.ai.activity.keyboard.windowMs > limits.ai.activity.keyboardWindowMsMax
  ) {
    return 'Keyboard activity window must be between 1 second and 5 minutes.';
  }
  if (
    draft.ai.activity.mouse.windowMs < limits.ai.activity.mouseWindowMsMin ||
    draft.ai.activity.mouse.windowMs > limits.ai.activity.mouseWindowMsMax
  ) {
    return 'Mouse activity window must be between 1 second and 5 minutes.';
  }
  if (
    draft.ai.transcription.remoteBaseUrl &&
    !isHttpUrl(draft.ai.transcription.remoteBaseUrl)
  ) {
    return 'Remote transcription URL must start with http:// or https://.';
  }
  if (draft.ai.vision.baseUrl && !isHttpUrl(draft.ai.vision.baseUrl)) {
    return 'Vision URL must start with http:// or https://.';
  }

  return null;
}

export function formatSessionCount(count: number): string {
  return `${count} session${count === 1 ? '' : 's'}`;
}

export function screenErrorMessage(error: unknown): string {
  if (
    typeof error === 'object' &&
    error !== null &&
    'message' in error &&
    typeof error.message === 'string'
  ) {
    return error.message;
  }

  return unknownErrorMessage(error);
}

function isHttpUrl(value: string): boolean {
  return value.startsWith('http://') || value.startsWith('https://');
}
