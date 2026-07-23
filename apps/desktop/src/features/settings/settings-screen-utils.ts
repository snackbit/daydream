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
  const windowProtectionError = validateWindowProtection(draft, limits);
  if (windowProtectionError) {
    return windowProtectionError;
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
    draft.ai.processing.maxParallelOcrJobs <
      limits.ai.processing.maxParallelOcrJobsMin ||
    draft.ai.processing.maxParallelOcrJobs >
      limits.ai.processing.maxParallelOcrJobsMax
  ) {
    return 'Max parallel OCR jobs must be between 1 and the CPU core count.';
  }
  if (
    draft.ai.processing.maxParallelVlmJobs <
      limits.ai.processing.maxParallelVlmJobsMin ||
    draft.ai.processing.maxParallelVlmJobs >
      limits.ai.processing.maxParallelVlmJobsMax
  ) {
    return 'Max parallel VLM jobs must be between 1 and the CPU core count.';
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

function validateWindowProtection(
  draft: AppSettings,
  limits: AppSettingsLimits,
): string | null {
  const settings = draft.record.windowProtection;
  if (
    settings.dwellMs < limits.record.windowProtectionDwellMsMin ||
    settings.dwellMs > limits.record.windowProtectionDwellMsMax
  ) {
    return 'Protected window dwell must be between 250 ms and 30 seconds.';
  }
  if (
    settings.mergeGapMs < limits.record.windowProtectionMergeGapMsMin ||
    settings.mergeGapMs > limits.record.windowProtectionMergeGapMsMax
  ) {
    return 'Protected window merge gap must be between 0 and 120 seconds.';
  }
  if (settings.rules.length > limits.record.windowProtectionRuleCountMax) {
    return 'Protected window rule count exceeds the maximum.';
  }

  const ids = new Set<string>();
  for (const rule of settings.rules) {
    const id = rule.id.trim();
    if (!id || ids.has(id)) {
      return 'Protected window rules need unique IDs.';
    }
    ids.add(id);
    if (!validText(rule.label, limits.record.windowProtectionTextMaxChars)) {
      return 'Protected window rule labels must be non-empty and short.';
    }
    if (
      rule.appIdentity &&
      !validText(rule.appIdentity, limits.record.windowProtectionTextMaxChars)
    ) {
      return 'Protected window app matchers must be short.';
    }
    if (
      rule.processName &&
      !validText(rule.processName, limits.record.windowProtectionTextMaxChars)
    ) {
      return 'Protected window process matchers must be short.';
    }
    const titleValue = rule.titleMatcher?.value.trim() ?? '';
    if (
      rule.enabled &&
      !rule.appIdentity?.trim() &&
      !rule.processName?.trim() &&
      !titleValue
    ) {
      return 'Enabled protected window rules need an app, process, or title matcher.';
    }
    if (rule.titleMatcher) {
      if (
        !validText(
          rule.titleMatcher.value,
          limits.record.windowProtectionTextMaxChars,
        )
      ) {
        return 'Protected window title matchers must be non-empty and short.';
      }
      if (rule.titleMatcher.kind === 'regex') {
        try {
          new RegExp(rule.titleMatcher.value);
        } catch {
          return 'Protected window title regex is invalid.';
        }
      }
    }
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

function validText(value: string, maxChars: number): boolean {
  const trimmed = value.trim();
  return trimmed.length > 0 && Array.from(trimmed).length <= maxChars;
}
