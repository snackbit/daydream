import type {
  BackgroundTaskDetail,
  BackgroundTaskPage,
  BackgroundTasksChanged,
  GetBackgroundTaskInput,
  ListBackgroundTasksInput,
  RetryProcessingUnitsInput,
  ProcessingJob,
  ListRecordingProcessingGroupsInput,
  RecordingProcessingGroupPage,
  GetRecordingProcessingLaneInput,
  RecordingProcessingLaneDetail,
  ControlRecordingProcessingGroupsInput,
  RecordingProcessingActionResult,
} from '@daydream/shared';
import { processingJob } from '@/features/ai/ocr/ocr-response-decoder';
import { reportFrontendFailure } from '@/lib/diagnostics/failure-logging';
import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';
import { listen } from '@/lib/diagnostics/listen';
import {
  decodeRecordingGroupPage,
  decodeRecordingLaneDetail,
  decodeRecordingActionResult,
} from './group-decoder';
import {
  decodeTaskChange,
  decodeTaskDetail,
  decodeTaskPage,
} from './task-decoder';

export async function listRecordingProcessingGroups(
  input?: ListRecordingProcessingGroupsInput,
): Promise<RecordingProcessingGroupPage> {
  return decodeResponse(
    'listRecordingProcessingGroups',
    await invoke<unknown>('listRecordingProcessingGroups', { input }),
    decodeRecordingGroupPage,
  );
}
export async function getRecordingProcessingLane(
  input: GetRecordingProcessingLaneInput,
): Promise<RecordingProcessingLaneDetail> {
  return decodeResponse(
    'getRecordingProcessingLane',
    await invoke<unknown>('getRecordingProcessingLane', { input }),
    decodeRecordingLaneDetail,
  );
}
export async function controlRecordingProcessingGroups(
  input: ControlRecordingProcessingGroupsInput,
): Promise<RecordingProcessingActionResult> {
  const result = decodeResponse(
    'controlRecordingProcessingGroups',
    await invoke<unknown>('controlRecordingProcessingGroups', { input }),
    decodeRecordingActionResult,
  );
  for (const action of result.results)
    if (!action.succeeded)
      reportFrontendFailure(
        'controlRecordingProcessingGroups',
        'execute',
        action.errorCode,
        { deduplicate: false },
      );
  return result;
}

export async function listBackgroundTasks(
  input?: ListBackgroundTasksInput,
): Promise<BackgroundTaskPage> {
  return decodeResponse(
    'listBackgroundTasks',
    await invoke<unknown>('listBackgroundTasks', { input }),
    decodeTaskPage,
  );
}
export async function getBackgroundTask(
  input: GetBackgroundTaskInput,
): Promise<BackgroundTaskDetail> {
  return decodeResponse(
    'getBackgroundTask',
    await invoke<unknown>('getBackgroundTask', { input }),
    decodeTaskDetail,
  );
}
export async function retryProcessingUnits(
  input: RetryProcessingUnitsInput,
): Promise<ProcessingJob> {
  return decodeResponse(
    'retryProcessingUnits',
    await invoke<unknown>('retryProcessingUnits', { input }),
    processingJob,
  );
}

export async function reprocessRecordingTranscription(input: {
  recordingId: string;
}): Promise<ProcessingJob> {
  return decodeResponse(
    'reprocessRecordingTranscription',
    await invoke<unknown>('reprocessRecordingTranscription', { input }),
    processingJob,
  );
}
export async function listenToBackgroundTasksChanged(
  handler: (change: BackgroundTasksChanged) => void,
): Promise<() => void> {
  return listen<unknown>('background-tasks-changed', (event) =>
    handler(decodeTaskChange(event.payload)),
  );
}
