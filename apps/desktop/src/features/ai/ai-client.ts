import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  AiModelAssetList,
  AiProviderStatus,
  CancelAiModelDownloadInput,
  DeleteAiModelInput,
  DeleteAiProviderSecretInput,
  DownloadAiModelInput,
  ListProcessingJobsInput,
  Notification,
  ProcessingJob,
  ProcessingJobActionInput,
  ProcessingJobList,
  RecordingActivity,
  RecordingAiResultInput,
  RecordingTranscript,
  SaveAiProviderSecretInput,
  SaveAiProviderSecretResult,
  SelfTestAiModelInput,
  StartRecordingAiProcessingInput,
  GetRecordingTranscriptInput,
} from '@daydream/shared';

export const aiModelsChangedEvent = 'ai-models-changed';

export async function listAiModels(): Promise<AiModelAssetList> {
  return invoke<AiModelAssetList>('listAiModels');
}

export async function downloadAiModel(
  input: DownloadAiModelInput,
): Promise<Notification> {
  return invoke<Notification>('downloadAiModel', { input });
}

export async function cancelAiModelDownload(
  input: CancelAiModelDownloadInput,
): Promise<Notification> {
  return invoke<Notification>('cancelAiModelDownload', { input });
}

export async function deleteAiModel(
  input: DeleteAiModelInput,
): Promise<AiModelAssetList> {
  return invoke<AiModelAssetList>('deleteAiModel', { input });
}

export async function selfTestAiModel(
  input: SelfTestAiModelInput,
): Promise<Notification> {
  return invoke<Notification>('selfTestAiModel', { input });
}

export async function getAiProviderStatus(): Promise<AiProviderStatus> {
  return invoke<AiProviderStatus>('getAiProviderStatus');
}

export async function saveAiProviderSecret(
  input: SaveAiProviderSecretInput,
): Promise<SaveAiProviderSecretResult> {
  return invoke<SaveAiProviderSecretResult>('saveAiProviderSecret', { input });
}

export async function deleteAiProviderSecret(
  input: DeleteAiProviderSecretInput,
): Promise<void> {
  return invoke<void>('deleteAiProviderSecret', { input });
}

export async function listProcessingJobs(
  input?: ListProcessingJobsInput,
): Promise<ProcessingJobList> {
  return invoke<ProcessingJobList>('listProcessingJobs', { input });
}

export async function pauseProcessingJob(
  input: ProcessingJobActionInput,
): Promise<ProcessingJob> {
  return invoke<ProcessingJob>('pauseProcessingJob', { input });
}

export async function resumeProcessingJob(
  input: ProcessingJobActionInput,
): Promise<ProcessingJob> {
  return invoke<ProcessingJob>('resumeProcessingJob', { input });
}

export async function cancelProcessingJob(
  input: ProcessingJobActionInput,
): Promise<ProcessingJob> {
  return invoke<ProcessingJob>('cancelProcessingJob', { input });
}

export async function retryProcessingJob(
  input: ProcessingJobActionInput,
): Promise<ProcessingJob> {
  return invoke<ProcessingJob>('retryProcessingJob', { input });
}

export async function startRecordingAiProcessing(
  input: StartRecordingAiProcessingInput,
): Promise<ProcessingJob> {
  return invoke<ProcessingJob>('startRecordingAiProcessing', { input });
}

export async function getRecordingTranscript(
  input: GetRecordingTranscriptInput,
): Promise<RecordingTranscript> {
  return invoke<RecordingTranscript>('getRecordingTranscript', { input });
}

export async function getRecordingActivity(
  input: RecordingAiResultInput,
): Promise<RecordingActivity> {
  return invoke<RecordingActivity>('getRecordingActivity', { input });
}

export async function listenToAiModelsChanged(
  handler: (models: AiModelAssetList) => void,
): Promise<() => void> {
  return listen<AiModelAssetList>(aiModelsChangedEvent, (event) => {
    handler(event.payload);
  });
}
