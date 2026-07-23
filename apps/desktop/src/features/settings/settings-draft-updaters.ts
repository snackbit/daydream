import type {
  AiActivitySourceSettings,
  AiActivityWindowSettings,
  AiOcrSettings,
  AiProcessingSettings,
  AiTranscriptionProviderSettings,
  AiVisionProviderSettings,
  AppSettings,
} from '@daydream/shared';

export type UpdateSettingsDraft = (
  updater: (draft: AppSettings) => AppSettings,
) => void;

export function updateAiTranscription(
  onUpdateDraft: UpdateSettingsDraft,
  updater: (
    transcription: AiTranscriptionProviderSettings,
  ) => AiTranscriptionProviderSettings,
) {
  onUpdateDraft((current) => ({
    ...current,
    ai: {
      ...current.ai,
      transcription: updater(current.ai.transcription),
    },
  }));
}

export function updateAiVision(
  onUpdateDraft: UpdateSettingsDraft,
  updater: (vision: AiVisionProviderSettings) => AiVisionProviderSettings,
) {
  onUpdateDraft((current) => ({
    ...current,
    ai: {
      ...current.ai,
      vision: updater(current.ai.vision),
    },
  }));
}

export function updateAiOcr(
  onUpdateDraft: UpdateSettingsDraft,
  updater: (ocr: AiOcrSettings) => AiOcrSettings,
) {
  onUpdateDraft((current) => ({
    ...current,
    ai: {
      ...current.ai,
      ocr: updater(current.ai.ocr),
    },
  }));
}

export function updateAiProcessing(
  onUpdateDraft: UpdateSettingsDraft,
  updater: (processing: AiProcessingSettings) => AiProcessingSettings,
) {
  onUpdateDraft((current) => ({
    ...current,
    ai: {
      ...current.ai,
      processing: updater(current.ai.processing),
    },
  }));
}

export function updateAiKeyboardActivity(
  onUpdateDraft: UpdateSettingsDraft,
  updater: (keyboard: AiActivityWindowSettings) => AiActivityWindowSettings,
) {
  onUpdateDraft((current) => ({
    ...current,
    ai: {
      ...current.ai,
      activity: {
        ...current.ai.activity,
        keyboard: updater(current.ai.activity.keyboard),
      },
    },
  }));
}

export function updateAiMouseActivity(
  onUpdateDraft: UpdateSettingsDraft,
  updater: (mouse: AiActivityWindowSettings) => AiActivityWindowSettings,
) {
  onUpdateDraft((current) => ({
    ...current,
    ai: {
      ...current.ai,
      activity: {
        ...current.ai.activity,
        mouse: updater(current.ai.activity.mouse),
      },
    },
  }));
}

export function updateAiWindowActivity(
  onUpdateDraft: UpdateSettingsDraft,
  updater: (windows: AiActivitySourceSettings) => AiActivitySourceSettings,
) {
  onUpdateDraft((current) => ({
    ...current,
    ai: {
      ...current.ai,
      activity: {
        ...current.ai.activity,
        windows: updater(current.ai.activity.windows),
      },
    },
  }));
}
