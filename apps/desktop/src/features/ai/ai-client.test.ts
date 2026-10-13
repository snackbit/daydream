import { beforeEach, expect, test, vi } from 'vitest';
import {
  aiModelsChangedEvent,
  aiProcessingChangedEvent,
  cancelAiModelDownload,
  cancelProcessingJob,
  deleteAiModel,
  deleteAiProviderSecret,
  downloadAiModel,
  getAiProviderStatus,
  getRecordingAiProcessingState,
  getRecordingActivity,
  getRecordingTranscript,
  getRecordingVisualObservations,
  listenToAiProcessingChanged,
  listenToAiModelsChanged,
  listAiModels,
  listProcessingJobs,
  pauseProcessingJob,
  retryProcessingJob,
  resumeProcessingJob,
  saveAiProviderSecret,
  selfTestAiModel,
  startRecordingAiProcessing,
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
  await startRecordingAiProcessing({ recordingId: 'recording-1' });
  await getRecordingAiProcessingState({ recordingId: 'recording-1' });
  await getRecordingTranscript({ recordingId: 'recording-1' });
  await getRecordingActivity({ recordingId: 'recording-1' });
  await getRecordingVisualObservations({ recordingId: 'recording-1' });

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
  expect(invokeMock).toHaveBeenNthCalledWith(14, 'startRecordingAiProcessing', {
    input: { recordingId: 'recording-1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(
    15,
    'getRecordingAiProcessingState',
    {
      input: { recordingId: 'recording-1' },
    },
  );
  expect(invokeMock).toHaveBeenNthCalledWith(16, 'getRecordingTranscript', {
    input: { recordingId: 'recording-1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(17, 'getRecordingActivity', {
    input: { recordingId: 'recording-1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(
    18,
    'getRecordingVisualObservations',
    {
      input: { recordingId: 'recording-1' },
    },
  );
});

test.each([undefined, []])(
  'scoped Studio overviews retain optional speech progress %s',
  async (transcriptionProgress) => {
    const bucket = {
      trackId: 'microphone-2',
      kind: 'speech',
      startOffsetMs: 1000,
      endOffsetMs: 2000,
      count: 3,
    };
    const state = {
      recordingId: 'recording-1',
      chunks: [],
      job: null,
      transcript: { recordingId: 'recording-1', tracks: [] },
      timelineOverview: [bucket],
      ...(transcriptionProgress ? { transcriptionProgress } : {}),
    };
    invokeMock.mockResolvedValue(state);
    const input = {
      recordingId: 'recording-1',
      startOffsetMs: 1000,
      endOffsetMs: 2000,
      bucketMs: 5000,
    };
    await expect(getRecordingAiProcessingState(input)).resolves.toEqual(state);
    expect(invokeMock).toHaveBeenCalledWith('getRecordingAiProcessingState', {
      input,
    });
    invokeMock.mockResolvedValue({
      ...state,
      timelineOverview: [{ ...bucket, count: 0 }],
    });
    await expect(getRecordingAiProcessingState(input)).rejects.toThrow();
  },
);

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

test('ai client listens for processing state updates', async () => {
  const unlisten = vi.fn();
  const handler = vi.fn();
  listenMock.mockImplementation((_eventName, listener) => {
    listener({
      payload: {
        chunks: [],
        job: null,
        recordingId: 'recording-1',
        transcript: { recordingId: 'recording-1', tracks: [] },
      },
    });
    return Promise.resolve(unlisten);
  });

  await expect(listenToAiProcessingChanged(handler)).resolves.toBe(unlisten);

  expect(listenMock).toHaveBeenCalledWith(
    aiProcessingChangedEvent,
    expect.any(Function),
  );
  expect(handler).toHaveBeenCalledWith({
    chunks: [],
    job: null,
    recordingId: 'recording-1',
    transcript: { recordingId: 'recording-1', tracks: [] },
  });
});
