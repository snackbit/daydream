import type { AppSettings } from '@daydream/shared';

export interface RemoteConsent {
  feature: string;
  destination: string;
  data: string;
}
function destination(
  settings: AppSettings,
  feature: 'summary' | 'embedding' | 'vision',
) {
  const registry = settings.ai.modelRegistry;
  const model = registry?.models.find(
    (model) => model.id === registry.bindings[feature],
  );
  const connection = registry?.connections.find(
    (connection) => connection.id === model?.connectionId,
  );
  const profile =
    feature === 'summary'
      ? settings.ai.recall.generation
      : feature === 'embedding'
        ? settings.ai.recall.embedding
        : settings.ai.vision;
  return {
    identity: JSON.stringify([
      model?.id,
      model?.model,
      connection?.provider,
      connection?.authMode,
      connection?.baseUrl,
      connection?.subscriptionId,
      profile.baseUrl,
      profile.model,
    ]),
    label: connection
      ? `${connection.name} · ${model?.name ?? 'Choose a model'} (${connection.baseUrl})`
      : `${profile.model ?? 'Choose a model'} · ${profile.baseUrl ?? 'Set a destination'}`,
  };
}
export function remoteConsentChanges(
  before: AppSettings,
  after: AppSettings,
): RemoteConsent[] {
  const checks = [
    {
      binding: 'summary',
      feature: 'Day summaries',
      was: before.ai.recall.generation.enabled,
      now: after.ai.recall.generation.enabled,
      data: 'Recognized text and audio transcripts. Screen images are excluded.',
    },
    {
      binding: 'embedding',
      feature: 'Semantic search',
      was:
        before.ai.recall.embedding.provider === 'openai-compatible' &&
        before.ai.recall.embedding.enabled,
      now:
        after.ai.recall.embedding.provider === 'openai-compatible' &&
        after.ai.recall.embedding.enabled,
      data: 'Recognized text, audio transcripts and search queries.',
    },
    {
      binding: 'vision',
      feature: 'Screen processing',
      was:
        before.ai.vision.provider === 'openai-compatible' &&
        ((before.ai.ocr.enabled && before.ai.ocr.engine === 'vision') ||
          before.ai.vision.observationsEnabled),
      now:
        after.ai.vision.provider === 'openai-compatible' &&
        ((after.ai.ocr.enabled && after.ai.ocr.engine === 'vision') ||
          after.ai.vision.observationsEnabled),
      data: 'Sampled screen images and recognized text for text recognition and screen observations.',
    },
  ] satisfies {
    binding: 'summary' | 'embedding' | 'vision';
    feature: string;
    was: boolean;
    now: boolean;
    data: string;
  }[];
  const result = checks
    .filter(
      (check) =>
        check.now &&
        (!check.was ||
          destination(before, check.binding).identity !==
            destination(after, check.binding).identity ||
          (check.binding === 'vision' &&
            ((!before.ai.vision.observationsEnabled &&
              after.ai.vision.observationsEnabled) ||
              (!(before.ai.ocr.enabled && before.ai.ocr.engine === 'vision') &&
                after.ai.ocr.enabled &&
                after.ai.ocr.engine === 'vision')))),
    )
    .map((check) => ({
      feature: check.feature,
      destination: destination(after, check.binding).label,
      data: check.data,
    }));
  const was = before.ai.transcription;
  const now = after.ai.transcription;
  if (
    now.provider === 'openai-compatible' &&
    (was.provider !== now.provider ||
      was.remoteBaseUrl !== now.remoteBaseUrl ||
      was.remoteModel !== now.remoteModel)
  )
    result.push({
      feature: 'Transcription',
      destination: `${now.remoteModel ?? 'Choose a model'} · ${now.remoteBaseUrl ?? 'Set a destination'}`,
      data: 'Recorded microphone and desktop audio.',
    });
  return result;
}
