import type {
  AiModelAsset,
  AiModelAssetList,
  BackgroundTask,
} from '@daydream/shared';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import { DownloadTaskRow } from './download-task-row';

const ai = vi.hoisted(() => ({
  listAiModels: vi.fn(),
  listenToAiModelsChanged: vi.fn(),
  downloadAiModel: vi.fn(),
  cancelAiModelDownload: vi.fn(),
  deleteAiModel: vi.fn(),
}));
vi.mock('../ai-client', () => ai);
const model: AiModelAsset = {
  id: 'synthetic-model',
  label: 'Synthetic model',
  provider: 'local-embedding',
  createdAtMs: 0,
  updatedAtMs: 0,
  checksum: null,
  checksumAlgorithm: 'sha256',
  downloadedBytes: 25,
  totalBytes: 100,
  sizeBytes: 100,
  downloadStatus: 'downloading',
  etag: null,
  errorCode: null,
  localPath: null,
  sourceUrl: 'https://synthetic.example/model.gguf',
  selfTestStatus: 'not-run',
  verificationStatus: 'pending',
};
const task: BackgroundTask = {
  id: 'model-download:synthetic-model',
  kind: 'model-download',
  title: 'Synthetic model',
  recordingId: null,
  modelId: model.id,
  status: 'running',
  phase: 'downloading',
  progress: 0.25,
  elapsedMs: 0,
  etaMs: null,
  nextRetryAtMs: null,
  inputOpen: false,
  errorCode: null,
  updatedAtMs: 0,
  stages: [],
};
beforeEach(() => {
  vi.resetAllMocks();
  ai.listenToAiModelsChanged.mockResolvedValue(() => undefined);
});
afterEach(cleanup);

test('shared model row reports lookup and subscription failures and recovers through Refresh', async () => {
  ai.listAiModels
    .mockRejectedValueOnce(
      new Error('CHECK constraint failed: private details'),
    )
    .mockResolvedValue({ models: [model] });
  ai.listenToAiModelsChanged
    .mockRejectedValueOnce(new Error('private subscription failure'))
    .mockResolvedValue(() => undefined);
  render(
    <ul>
      <DownloadTaskRow task={task} />
    </ul>,
  );
  await screen.findByRole('button', { name: 'Refresh download status' });
  expect(
    screen.queryByText(/CHECK constraint|private subscription/),
  ).toBeNull();
  await page.getByRole('button', { name: 'Refresh download status' }).click();
  await screen.findByRole('group', { name: model.label });
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
  expect(
    screen.getByRole('progressbar', {
      name: `${model.label} download progress`,
    }),
  ).toHaveAttribute('aria-valuenow', '25');
});

test('a newer model event takes ownership over a late initial snapshot without rebuilding the subscription', async () => {
  let resolveSnapshot: ((value: AiModelAssetList) => void) | undefined;
  let publish: ((value: AiModelAssetList) => void) | undefined;
  ai.listAiModels.mockImplementation(
    () =>
      new Promise((resolve) => {
        resolveSnapshot = resolve;
      }),
  );
  ai.listenToAiModelsChanged.mockImplementation(
    async (listener: (value: AiModelAssetList) => void) => {
      publish = listener;
      return () => undefined;
    },
  );
  render(
    <ul>
      <DownloadTaskRow task={task} />
    </ul>,
  );
  await waitFor(() =>
    expect(ai.listenToAiModelsChanged).toHaveBeenCalledTimes(1),
  );
  if (!publish || !resolveSnapshot)
    throw new Error('Model read boundaries were not registered');
  publish({
    models: [{ ...model, downloadStatus: 'paused', downloadedBytes: 50 }],
  });
  await screen.findByRole('button', { name: 'Resume' });
  const completeSnapshot = resolveSnapshot;
  await act(async () => {
    completeSnapshot({ models: [model] });
  });
  expect(screen.getByRole('button', { name: 'Resume' })).toBeTruthy();
  expect(
    screen.getByRole('progressbar', {
      name: `${model.label} download progress`,
    }),
  ).toHaveAttribute('aria-valuenow', '50');
  expect(ai.listenToAiModelsChanged).toHaveBeenCalledTimes(1);
});
