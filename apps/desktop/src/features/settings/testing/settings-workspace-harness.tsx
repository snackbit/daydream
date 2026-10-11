import type { AiModelAsset, AppSettingsState } from '@daydream/shared';
import { cleanup } from '@testing-library/react';
import { vi } from 'vitest';
import { page } from 'vitest/browser';
import '@/index.css';
import { emptyRegistry, updateRegistry } from '../models/model-registry';
import { useSettingsStore } from '../settings-store';
import {
  defaultSettingsState,
  scene,
  aiModels,
  availableInputActivityStatus,
} from './settings-browser-fixtures';
import {
  settings,
  recorder,
  ai,
  transfer,
  recall,
  maintenance,
  accounts,
  blocker,
} from './settings-workspace-mocks';

export let saved: AppSettingsState;
export {
  settings,
  recorder,
  ai,
  transfer,
  recall,
  maintenance,
  accounts,
  blocker,
};
export function localAsset(
  id: string,
  label: string,
  downloadStatus: AiModelAsset['downloadStatus'] = 'missing',
): AiModelAsset {
  return {
    ...aiModels[0],
    id,
    label,
    provider: 'local-embedding',
    downloadStatus,
  };
}
export function configureAccounts() {
  saved.settings = updateRegistry(saved.settings, () => ({
    ...emptyRegistry(),
    connections: [
      {
        id: 'synthetic-api',
        name: 'Synthetic API',
        provider: 'openai-compatible',
        authMode: 'none',
        baseUrl: 'https://example.test/v1',
        apiKeyRef: null,
        subscriptionId: null,
        protocol: 'responses',
        tokenLimit: 'modern',
      },
    ],
    models: [
      {
        id: 'synthetic-model',
        name: 'Synthetic model',
        model: 'synthetic',
        connectionId: 'synthetic-api',
        capabilities: ['text', 'vision', 'embedding'],
        embeddingDimensions: 384,
        embeddingConvention: 'plain',
      },
    ],
    bindings: {
      summary: 'synthetic-model',
      embedding: 'synthetic-model',
      vision: 'synthetic-model',
    },
  }));
}
export function setupWorkspace() {
  vi.resetAllMocks();
  saved = structuredClone(defaultSettingsState);
  saved.settings.ai.modelRegistry = emptyRegistry();
  useSettingsStore.getState().invalidatePreferences();
  useSettingsStore.setState(useSettingsStore.getInitialState(), true);
  blocker.status = 'idle';
  settings.getAppSettings.mockImplementation(async () => saved);
  settings.saveAppSettings.mockImplementation(
    async ({ settings: next, credentials }) => {
      const normalized = structuredClone(next);
      if (credentials?.transcriptionKey)
        normalized.ai.transcription.remoteApiKeyRef = 'synthetic-encrypted-ref';
      for (const connection of normalized.ai.modelRegistry?.connections ?? [])
        if (credentials?.modelKeys?.[connection.id])
          connection.apiKeyRef = `synthetic-encrypted-${connection.id}`;
      saved = { ...saved, settings: normalized };
      return saved;
    },
  );
  settings.resetAppSettings.mockImplementation(async () => ({
    ...saved,
    settings: saved.defaults,
  }));
  settings.previewAppSettingsStorageCleanup.mockResolvedValue({
    recordingCount: 0,
    sizeBytes: 0,
  });
  settings.getInputActivityStatus.mockResolvedValue(
    availableInputActivityStatus,
  );
  settings.repairInputActivityPermissions.mockResolvedValue({
    message: 'Synthetic permission repair completed.',
    success: true,
    status: availableInputActivityStatus,
  });
  recorder.getRecorderScene.mockResolvedValue(scene);
  recorder.getWindowProtectionStatus.mockResolvedValue({
    message: 'Synthetic window provider available.',
    platform: 'linux',
    provider: 'linux-x11',
    status: 'available',
  });
  recorder.listProtectableWindows.mockResolvedValue({
    windows: [],
    status: {
      message: 'Synthetic window provider available.',
      platform: 'linux',
      provider: 'linux-x11',
      status: 'available',
    },
  });
  ai.listAiModels.mockResolvedValue({
    models: [
      ...aiModels,
      localAsset('multilingual-e5-small-q8_0', 'Multilingual E5'),
      localAsset('bge-small-en-v1.5-q8_0', 'BGE English'),
    ],
  });
  ai.getAiProviderStatus.mockResolvedValue({
    localTranscription: {
      availableLanguages: [{ id: 'de', label: 'German' }],
      availableBackends: [
        { id: 'cpu', available: true, reason: null },
        { id: 'vulkan', available: true, reason: null },
        { id: 'cuda', available: false, reason: 'Not compiled.' },
      ],
    },
    ocr: { enabled: true },
    remoteTranscription: 'not-configured',
    vision: 'disabled',
  });
  ai.listenToAiModelsChanged.mockResolvedValue(() => undefined);
  recall.listenToRecallStateChanged.mockResolvedValue(() => undefined);
  recall.getRecallStatus.mockResolvedValue({
    privacyRevision: 0,
    indexedEvidenceCount: 0,
    pendingEvidenceCount: 0,
    embeddingReady: false,
    generationReady: false,
    embeddingUnavailableReason: 'recall_embedding_model_missing',
    generationUnavailableReason: 'Disabled',
    jobs: [],
  });
  maintenance.getMaintenanceStatus.mockResolvedValue(null);
  maintenance.previewMaintenance.mockImplementation(async ({ action }) => ({
    action,
    token: 'synthetic-impact',
    recordingCount: 2,
    libraryRowCount: 5,
    modelCount: 1,
    fileCount: 0,
    sizeBytes: 0,
  }));
  maintenance.executeMaintenance.mockResolvedValue({
    action: 'reset-preferences',
    operationId: 'synthetic-operation',
    phase: 'completed',
    error: null,
  });
  accounts.signOutChatGpt.mockResolvedValue(true);
  accounts.getChatGptConnection.mockResolvedValue({
    id: 'synthetic-subscription',
    status: 'connected',
    label: 'Synthetic account',
    models: [],
  });
  accounts.prepareChatGptConnection.mockResolvedValue('synthetic-subscription');
  accounts.signInChatGpt.mockResolvedValue({
    id: 'synthetic-subscription',
    status: 'connected',
    label: 'Synthetic account',
    models: [{ slug: 'synthetic-text', displayName: 'Synthetic text' }],
  });
  accounts.testConfiguredModel.mockResolvedValue({
    fingerprint: 'synthetic',
    checks: [
      {
        capability: 'text',
        status: 'succeeded',
        message: 'Passed',
        elapsedMs: 1,
        dimensions: null,
      },
    ],
  });
}
export async function cleanupWorkspace() {
  cleanup();
  delete document.documentElement.dataset.theme;
  await page.viewport(1440, 960);
}
export async function selectSetting(name: string, value: string) {
  await page.getByRole('combobox', { name, exact: true }).click();
  await page.getByRole('option', { name: value, exact: true }).click();
}
