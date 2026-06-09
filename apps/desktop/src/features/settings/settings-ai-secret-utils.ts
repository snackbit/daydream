import type { AppSettings } from '@daydream/shared';

export function settingsForCleanupPreview(settings: AppSettings): AppSettings {
  let nextSettings = settings;

  if (settings.ai.transcription.provider !== 'openai-compatible') {
    nextSettings = {
      ...nextSettings,
      ai: {
        ...nextSettings.ai,
        transcription: {
          ...nextSettings.ai.transcription,
          remoteApiKeyRef: null,
        },
      },
    };
  }

  if (settings.ai.vision.provider !== 'openai-compatible') {
    nextSettings = {
      ...nextSettings,
      ai: {
        ...nextSettings.ai,
        vision: {
          ...nextSettings.ai.vision,
          apiKeyRef: null,
        },
      },
    };
  }

  return nextSettings;
}

export function collectSupersededAiSecretRefs(
  savedSettings: AppSettings | null,
  nextSettings: AppSettings,
): string[] {
  if (!savedSettings) {
    return [];
  }

  const savedRefs = [
    savedSettings.ai.transcription.remoteApiKeyRef,
    savedSettings.ai.vision.apiKeyRef,
  ];
  const nextRefs = new Set([
    nextSettings.ai.transcription.remoteApiKeyRef,
    nextSettings.ai.vision.apiKeyRef,
  ]);

  return Array.from(
    new Set(
      savedRefs.filter(
        (secretRef): secretRef is string =>
          Boolean(secretRef) && !nextRefs.has(secretRef),
      ),
    ),
  );
}
