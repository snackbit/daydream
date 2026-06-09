import { beforeEach, expect, test, vi } from 'vitest';
import {
  cancelProcessingJob,
  deleteAiModel,
  deleteAiProviderSecret,
  downloadAiModel,
  getAiProviderStatus,
  listAiModels,
  listProcessingJobs,
  pauseProcessingJob,
  retryProcessingJob,
  resumeProcessingJob,
  saveAiProviderSecret,
  selfTestAiModel,
} from './ai-client';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockResolvedValue({});
});

test('ai client uses typed camelCase command names', async () => {
  await listAiModels();
  await downloadAiModel({ modelId: 'small' });
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
  expect(invokeMock).toHaveBeenNthCalledWith(3, 'deleteAiModel', {
    input: { modelId: 'small' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(4, 'selfTestAiModel', {
    input: { modelId: 'small' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(5, 'getAiProviderStatus');
  expect(invokeMock).toHaveBeenNthCalledWith(6, 'saveAiProviderSecret', {
    input: { plaintext: 'sk-test', role: 'transcription' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(7, 'deleteAiProviderSecret', {
    input: { secretRef: 'ai.secret.transcription.1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(8, 'listProcessingJobs', {
    input: { recordingId: 'recording-1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(9, 'pauseProcessingJob', {
    input: { jobId: 'job-1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(10, 'resumeProcessingJob', {
    input: { jobId: 'job-1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(11, 'cancelProcessingJob', {
    input: { jobId: 'job-1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(12, 'retryProcessingJob', {
    input: { jobId: 'job-1' },
  });
});
