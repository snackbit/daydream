import { beforeEach, expect, test, vi } from 'vitest';
import {
  aiModelsChangedEvent,
  cancelAiModelDownload,
  cancelProcessingJob,
  deleteAiModel,
  deleteAiProviderSecret,
  downloadAiModel,
  getAiProviderStatus,
  listenToAiModelsChanged,
  listAiModels,
  listProcessingJobs,
  pauseProcessingJob,
  retryProcessingJob,
  resumeProcessingJob,
  saveAiProviderSecret,
  selfTestAiModel,
} from './ai-client';

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockResolvedValue({});
  listenMock.mockReset();
});

test('ai client uses typed camelCase command names', async () => {
  await listAiModels();
  await downloadAiModel({ modelId: 'small' });
  await cancelAiModelDownload({ modelId: 'small' });
  await deleteAiModel({ modelId: 'small' });
  await selfTestAiModel({ modelId: 'small' });
  await getAiProviderStatus();
  await saveAiProviderSecret({ plaintext: 'sk-test', role: 'transcription' });
  await deleteAiProviderSecret({ secretRef: 'ai.secret.transcription.1' });
  await listProcessingJobs({ recordingId: 'recording-1' });
  await pauseProcessingJob({ jobId: 'job-1' });
  await resumeProcessingJob({ jobId: 'job-1' });
  await cancelProcessingJob({ jobId: 'job-1' });
  await retryProcessingJob({ jobId: 'job-1' });

  expect(invokeMock).toHaveBeenNthCalledWith(1, 'listAiModels');
  expect(invokeMock).toHaveBeenNthCalledWith(2, 'downloadAiModel', {
    input: { modelId: 'small' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(3, 'cancelAiModelDownload', {
    input: { modelId: 'small' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(4, 'deleteAiModel', {
    input: { modelId: 'small' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(5, 'selfTestAiModel', {
    input: { modelId: 'small' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(6, 'getAiProviderStatus');
  expect(invokeMock).toHaveBeenNthCalledWith(7, 'saveAiProviderSecret', {
    input: { plaintext: 'sk-test', role: 'transcription' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(8, 'deleteAiProviderSecret', {
    input: { secretRef: 'ai.secret.transcription.1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(9, 'listProcessingJobs', {
    input: { recordingId: 'recording-1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(10, 'pauseProcessingJob', {
    input: { jobId: 'job-1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(11, 'resumeProcessingJob', {
    input: { jobId: 'job-1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(12, 'cancelProcessingJob', {
    input: { jobId: 'job-1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(13, 'retryProcessingJob', {
    input: { jobId: 'job-1' },
  });
});

test('ai client listens for full model list updates', async () => {
  const unlisten = vi.fn();
  const handler = vi.fn();
  listenMock.mockImplementation((_eventName, listener) => {
    listener({ payload: { models: [] } });
    return Promise.resolve(unlisten);
  });

  await expect(listenToAiModelsChanged(handler)).resolves.toBe(unlisten);

  expect(listenMock).toHaveBeenCalledWith(
    aiModelsChangedEvent,
    expect.any(Function),
  );
  expect(handler).toHaveBeenCalledWith({ models: [] });
});
