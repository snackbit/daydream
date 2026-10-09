import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
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
  return decodeRecordingGroupPage(
    await invoke<unknown>('listRecordingProcessingGroups', { input }),
  );
}
export async function getRecordingProcessingLane(
  input: GetRecordingProcessingLaneInput,
): Promise<RecordingProcessingLaneDetail> {
  return decodeRecordingLaneDetail(
    await invoke<unknown>('getRecordingProcessingLane', { input }),
  );
}
export async function controlRecordingProcessingGroups(
  input: ControlRecordingProcessingGroupsInput,
): Promise<RecordingProcessingActionResult> {
  return decodeRecordingActionResult(
    await invoke<unknown>('controlRecordingProcessingGroups', { input }),
  );
}

export async function listBackgroundTasks(
  input?: ListBackgroundTasksInput,
): Promise<BackgroundTaskPage> {
  return decodeTaskPage(
    await invoke<unknown>('listBackgroundTasks', { input }),
  );
}
export async function getBackgroundTask(
  input: GetBackgroundTaskInput,
): Promise<BackgroundTaskDetail> {
  return decodeTaskDetail(
    await invoke<unknown>('getBackgroundTask', { input }),
  );
}
export async function retryProcessingUnits(
  input: RetryProcessingUnitsInput,
): Promise<ProcessingJob> {
  return processingJob(
    await invoke<unknown>('retryProcessingUnits', { input }),
  );
}

export async function reprocessRecordingTranscription(input: {
  recordingId: string;
}): Promise<ProcessingJob> {
  return processingJob(
    await invoke<unknown>('reprocessRecordingTranscription', { input }),
  );
}
export async function listenToBackgroundTasksChanged(
  handler: (change: BackgroundTasksChanged) => void,
): Promise<() => void> {
  return listen<unknown>('background-tasks-changed', (event) =>
    handler(decodeTaskChange(event.payload)),
  );
}
