import type {
  AiModelAssetList,
  BackgroundTasksChanged,
  AiProviderStatus,
  CancelAiModelDownloadInput,
  DeleteAiModelInput,
  DeleteAiProviderSecretInput,
  DownloadAiModelInput,
  GetRecordingAiProcessingStateInput,
  ListProcessingJobsInput,
  Notification,
  ProcessingJob,
  ProcessingJobActionInput,
  ProcessingJobList,
  RecordingAiProcessingState,
  RecordingActivity,
  RecordingAiResultInput,
  RecordingTranscript,
  RecordingVisualObservations,
  SaveAiProviderSecretInput,
  SaveAiProviderSecretResult,
  SelfTestAiModelInput,
  StartRecordingAiProcessingInput,
  GetRecordingTranscriptInput,
} from '@daydream/shared';
import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';
import { listen } from '@/lib/diagnostics/listen';
import { decodeTranscriptionProgress } from './transcription-progress-decoder';

export {
  pauseAiModelDownload,
  resumeAiModelDownload,
  removeAiModelPartial,
} from './models/model-transfer-client';

export const aiModelsChangedEvent = 'ai-models-changed';
export const aiProcessingChangedEvent = 'ai-processing-changed';

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

export async function getRecordingAiProcessingState(
  input: GetRecordingAiProcessingStateInput,
): Promise<RecordingAiProcessingState> {
  const state = await invoke<RecordingAiProcessingState>(
    'getRecordingAiProcessingState',
    {
      input,
    },
  );
  return state.transcriptionProgress === undefined
    ? state
    : {
        ...state,
        transcriptionProgress: decodeResponse(
          'getRecordingAiProcessingState',
          state.transcriptionProgress,
          decodeTranscriptionProgress,
        ),
      };
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

export async function getRecordingVisualObservations(
  input: RecordingAiResultInput,
): Promise<RecordingVisualObservations> {
  return invoke<RecordingVisualObservations>('getRecordingVisualObservations', {
    input,
  });
}

export async function listenToAiModelsChanged(
  handler: (models: AiModelAssetList) => void,
): Promise<() => void> {
  return listen<AiModelAssetList>(aiModelsChangedEvent, (event) => {
    handler(event.payload);
  });
}

export async function listenToAiProcessingChanged(
  handler: (state: RecordingAiProcessingState | BackgroundTasksChanged) => void,
): Promise<() => void> {
  return listen<BackgroundTasksChanged>(aiProcessingChangedEvent, (event) => {
    handler(event.payload);
  });
}
