import { invoke } from '@tauri-apps/api/core';
import type {
  AiModelAssetList,
  AiProviderStatus,
  DeleteAiModelInput,
  DeleteAiProviderSecretInput,
  DownloadAiModelInput,
  ListProcessingJobsInput,
  Notification,
  ProcessingJob,
  ProcessingJobActionInput,
  ProcessingJobList,
  SaveAiProviderSecretInput,
  SaveAiProviderSecretResult,
  SelfTestAiModelInput,
} from '@daydream/shared';

export async function listAiModels(): Promise<AiModelAssetList> {
  return invoke<AiModelAssetList>('listAiModels');
}

export async function downloadAiModel(
  input: DownloadAiModelInput,
): Promise<Notification> {
  return invoke<Notification>('downloadAiModel', { input });
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
