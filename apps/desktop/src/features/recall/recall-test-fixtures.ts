import type { RecallSettings } from '@daydream/shared';

export function recallSettingsFixture(): RecallSettings {
  return {
    embedding: {
      provider: 'local',
      localModelId: 'multilingual-e5-small-q8_0',
      localBackend: 'cpu',
      dimensions: 384,
      convention: 'plain',
      enabled: false,
      baseUrl: null,
      model: null,
      authMode: 'api-key',
      apiKeyRef: null,
    },
    generation: {
      enabled: false,
      baseUrl: null,
      model: null,
      authMode: 'api-key',
      apiKeyRef: null,
    },
    autoGenerate: true,
  };
}
