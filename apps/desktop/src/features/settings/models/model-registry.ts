import type {
  AiModelRegistry,
  AppSettings,
  ModelCapability,
} from '@daydream/shared';

export function emptyRegistry(): AiModelRegistry {
  return {
    version: 1,
    connections: [],
    models: [],
    bindings: { embedding: null, summary: null, vision: null },
  };
}
export function updateRegistry(
  settings: AppSettings,
  update: (registry: AiModelRegistry) => AiModelRegistry,
): AppSettings {
  const registry = update(settings.ai.modelRegistry ?? emptyRegistry());
  const selected = (id: string | null) => {
    const model = registry.models.find((model) => model.id === id);
    const connection = registry.connections.find(
      (connection) => connection.id === model?.connectionId,
    );
    return model && connection ? { model, connection } : null;
  };
  const embedding = selected(registry.bindings.embedding),
    summary = selected(registry.bindings.summary),
    vision = selected(registry.bindings.vision);
  const fields = ({
    model,
    connection,
  }: NonNullable<ReturnType<typeof selected>>) => ({
    baseUrl: connection.baseUrl || null,
    model: model.model || null,
    authMode:
      connection.authMode === 'none' ? ('none' as const) : ('api-key' as const),
    apiKeyRef:
      connection.apiKeyRef ??
      (connection.subscriptionId
        ? `ai.secret.chatgpt.${connection.subscriptionId}`
        : null),
  });
  return {
    ...settings,
    ai: {
      ...settings.ai,
      modelRegistry: registry,
      recall: {
        ...settings.ai.recall,
        embedding: embedding
          ? {
              ...settings.ai.recall.embedding,
              ...fields(embedding),
              provider: 'openai-compatible',
              dimensions:
                embedding.model.embeddingDimensions ??
                settings.ai.recall.embedding.dimensions,
              convention: embedding.model.embeddingConvention,
            }
          : {
              ...settings.ai.recall.embedding,
              ...(settings.ai.recall.embedding.provider === 'openai-compatible'
                ? { baseUrl: null, model: null, apiKeyRef: null }
                : {}),
            },
        generation: {
          ...settings.ai.recall.generation,
          ...(summary
            ? fields(summary)
            : { baseUrl: null, model: null, apiKeyRef: null }),
        },
      },
      vision: {
        ...settings.ai.vision,
        ...(vision
          ? fields(vision)
          : { baseUrl: null, model: null, apiKeyRef: null }),
      },
    },
  };
}
export function selectableModels(
  registry: AiModelRegistry,
  capability: ModelCapability,
) {
  return registry.models.filter((model) =>
    model.capabilities.includes(capability),
  );
}
export function selectedFeatures(registry: AiModelRegistry, modelId: string) {
  return Object.entries(registry.bindings)
    .filter(([, id]) => id === modelId)
    .map(([feature]) => feature);
}
