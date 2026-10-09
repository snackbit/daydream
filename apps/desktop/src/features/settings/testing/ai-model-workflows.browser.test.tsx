import type { AiModelAsset } from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import { AiModelCacheCard } from '../components/ai-model-cache-card';
import {
  useSettingsAiController,
  settingsAiModelActions,
} from '../use-settings-ai-controller';

const ai = vi.hoisted(() => ({
  listAiModels: vi.fn(),
  getAiProviderStatus: vi.fn(),
  listenToAiModelsChanged: vi.fn(),
  downloadAiModel: vi.fn(),
  cancelAiModelDownload: vi.fn(),
  deleteAiModel: vi.fn(),
  deleteAiProviderSecret: vi.fn(),
}));
const settings = vi.hoisted(() => ({
  getInputActivityStatus: vi.fn(),
  repairInputActivityPermissions: vi.fn(),
}));
const notifications = vi.hoisted(() => ({ getNotifications: vi.fn() }));
vi.mock('@/features/ai/ai-client', () => ai);
vi.mock('../settings-client', () => settings);
vi.mock('@/features/notifications/notifications-client', () => notifications);

function model(downloadStatus: AiModelAsset['downloadStatus']): AiModelAsset {
  return {
    id: 'synthetic-model',
    label: 'Synthetic model',
    provider: 'local-whisper-cpp',
    createdAtMs: 0,
    updatedAtMs: 0,
    checksum: null,
    checksumAlgorithm: null,
    downloadedBytes: 0,
    totalBytes: 100,
    sizeBytes: 100,
    downloadStatus,
    etag: null,
    errorCode: null,
    localPath: null,
    sourceUrl: 'https://example.test/model.bin',
    selfTestStatus: 'not-run',
    verificationStatus: 'unknown',
  };
}

function ModelSettingsWorkflow() {
  const controller = useSettingsAiController({
    loadNotifications: notifications.getNotifications,
    selectedAiModelId: 'synthetic-model',
  });
  function download(modelId: string) {
    void controller.runAiModelAction(() =>
      settingsAiModelActions.downloadAiModel({ modelId }),
    );
  }
  function cancel(modelId: string) {
    void controller.runAiModelAction(() =>
      settingsAiModelActions.cancelAiModelDownload({ modelId }),
    );
  }
  function remove(modelId: string) {
    void controller.runAiModelAction(() =>
      settingsAiModelActions.deleteAiModel({ modelId }),
    );
  }
  return (
    <>
      <AiModelCacheCard
        models={controller.aiModels}
        onDownloadModel={download}
        onCancelModelDownload={cancel}
        onDeleteModel={remove}
      />
      {controller.aiActionError ? (
        <p role="alert">{controller.aiActionError}</p>
      ) : null}
    </>
  );
}

beforeEach(() => {
  vi.resetAllMocks();
  ai.listenToAiModelsChanged.mockResolvedValue(() => undefined);
  ai.getAiProviderStatus.mockResolvedValue(null);
  settings.getInputActivityStatus.mockResolvedValue(null);
  notifications.getNotifications.mockResolvedValue(undefined);
});
afterEach(cleanup);

test('SET-01 local model download can be cancelled then retried and deleted once available', async () => {
  let current = model('missing');
  ai.listAiModels.mockImplementation(async () => ({ models: [current] }));
  ai.downloadAiModel.mockImplementation(async () => {
    current = model(
      current.downloadStatus === 'missing' ? 'downloading' : 'available',
    );
  });
  ai.cancelAiModelDownload.mockImplementation(async () => {
    current = model('cancelled');
  });
  ai.deleteAiModel.mockImplementation(async () => {
    current = model('missing');
  });
  render(<ModelSettingsWorkflow />);
  await screen.findByRole('button', { name: 'Download' });
  await page.getByRole('button', { name: 'Download', exact: true }).click();
  await screen.findByRole('button', { name: 'Cancel' });
  expect(ai.downloadAiModel).toHaveBeenCalledWith({
    modelId: 'synthetic-model',
  });
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await screen.findByRole('button', { name: 'Retry' });
  expect(ai.cancelAiModelDownload).toHaveBeenCalledWith({
    modelId: 'synthetic-model',
  });
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  await screen.findByRole('button', { name: 'Delete' });
  await page.getByRole('button', { name: 'Delete', exact: true }).click();
  await screen.findByRole('button', { name: 'Download' });
  expect(ai.deleteAiModel).toHaveBeenCalledWith({ modelId: 'synthetic-model' });
  expect(notifications.getNotifications).toHaveBeenCalledTimes(4);
});

test('SET-02 failed model deletion keeps the available model and recovers on retry', async () => {
  let current = model('available');
  ai.listAiModels.mockImplementation(async () => ({ models: [current] }));
  ai.deleteAiModel
    .mockRejectedValueOnce(new Error('Model file is busy.'))
    .mockImplementationOnce(async () => {
      current = model('missing');
    });
  render(<ModelSettingsWorkflow />);
  await screen.findByRole('button', { name: 'Delete' });
  await page.getByRole('button', { name: 'Delete', exact: true }).click();
  expect(await screen.findByRole('alert')).toHaveProperty(
    'textContent',
    'The model action could not be completed. Your saved partial data can be retried.',
  );
  expect(screen.getByText('Downloaded')).toBeTruthy();
  await page.getByRole('button', { name: 'Delete', exact: true }).click();
  await screen.findByRole('button', { name: 'Download' });
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
  expect(ai.deleteAiModel).toHaveBeenCalledTimes(2);
});
