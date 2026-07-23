import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  AudioSourceList,
  CaptureSourceList,
  PauseRecordingInput,
  ProtectableWindowList,
  RecorderPerformanceStats,
  RecorderPreviewFrame,
  RecorderPreviewStatus,
  RecorderRecordingStats,
  RecorderScene,
  RecorderStatus,
  ResumeRecordingInput,
  SaveRecorderSceneInput,
  SelectCaptureSourcesInput,
  StartRecorderPreviewInput,
  StartRecordingInput,
  StopRecordingInput,
  WindowProtectionStatus,
} from '@daydream/shared';

export const recorderStatusChangedEvent = 'recorder-status-changed';
export const recorderPreviewFrameEvent = 'recorder-preview-frame';

export async function getRecorderStatus(): Promise<RecorderStatus> {
  return invoke<RecorderStatus>('getRecorderStatus');
}

export async function getRecorderPerformanceStats(): Promise<RecorderPerformanceStats> {
  return invoke<RecorderPerformanceStats>('getRecorderPerformanceStats');
}

export async function getRecorderRecordingStats(): Promise<RecorderRecordingStats | null> {
  return invoke<RecorderRecordingStats | null>('getRecorderRecordingStats');
}

export async function listProtectableWindows(): Promise<ProtectableWindowList> {
  return invoke<ProtectableWindowList>('listProtectableWindows');
}

export async function getWindowProtectionStatus(): Promise<WindowProtectionStatus> {
  return invoke<WindowProtectionStatus>('getWindowProtectionStatus');
}

export async function getCaptureSources(): Promise<CaptureSourceList> {
  return invoke<CaptureSourceList>('getCaptureSources');
}

export async function getRecorderScene(): Promise<RecorderScene | null> {
  return invoke<RecorderScene | null>('getRecorderScene');
}

export async function selectCaptureSources(
  input?: SelectCaptureSourcesInput,
): Promise<CaptureSourceList> {
  return input
    ? invoke<CaptureSourceList>('selectCaptureSources', { input })
    : invoke<CaptureSourceList>('selectCaptureSources');
}

export async function getAudioSources(): Promise<AudioSourceList> {
  return invoke<AudioSourceList>('getAudioSources');
}

export async function saveRecorderScene(
  input: SaveRecorderSceneInput,
): Promise<RecorderScene> {
  return invoke<RecorderScene>('saveRecorderScene', { input });
}

export async function getRecorderPreviewStatus(): Promise<RecorderPreviewStatus> {
  return invoke<RecorderPreviewStatus>('getRecorderPreviewStatus');
}

export async function startRecorderPreview(
  input: StartRecorderPreviewInput,
): Promise<RecorderPreviewStatus> {
  return invoke<RecorderPreviewStatus>('startRecorderPreview', { input });
}

export async function stopRecorderPreview(): Promise<RecorderPreviewStatus> {
  return invoke<RecorderPreviewStatus>('stopRecorderPreview');
}

export async function startRecording(
  input: StartRecordingInput,
): Promise<RecorderStatus> {
  return invoke<RecorderStatus>('startRecording', { input });
}

export async function pauseRecording(
  input: PauseRecordingInput,
): Promise<RecorderStatus> {
  return invoke<RecorderStatus>('pauseRecording', { input });
}

export async function resumeRecording(
  input: ResumeRecordingInput,
): Promise<RecorderStatus> {
  return invoke<RecorderStatus>('resumeRecording', { input });
}

export async function stopRecording(
  input: StopRecordingInput,
): Promise<RecorderStatus> {
  return invoke<RecorderStatus>('stopRecording', { input });
}

export async function listenToRecorderStatusChanged(
  handler: (status: RecorderStatus) => void,
): Promise<() => void> {
  return listen<RecorderStatus>(recorderStatusChangedEvent, (event) => {
    handler(event.payload);
  });
}

export async function listenToRecorderPreviewFrames(
  handler: (frame: RecorderPreviewFrame) => void,
): Promise<() => void> {
  return listen<RecorderPreviewFrame>(recorderPreviewFrameEvent, (event) => {
    handler(event.payload);
  });
}
