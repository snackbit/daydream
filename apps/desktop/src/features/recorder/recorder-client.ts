import { invoke } from '@tauri-apps/api/core';
import type {
  AudioSourceList,
  CaptureSourceList,
  PauseRecordingInput,
  RecorderScene,
  RecorderStatus,
  ResumeRecordingInput,
  SaveRecorderSceneInput,
  StartRecordingInput,
  StopRecordingInput,
} from '@daydream/shared';

export async function getRecorderStatus(): Promise<RecorderStatus> {
  return invoke<RecorderStatus>('getRecorderStatus');
}

export async function getCaptureSources(): Promise<CaptureSourceList> {
  return invoke<CaptureSourceList>('getCaptureSources');
}

export async function selectCaptureSources(): Promise<CaptureSourceList> {
  return invoke<CaptureSourceList>('selectCaptureSources');
}

export async function getAudioSources(): Promise<AudioSourceList> {
  return invoke<AudioSourceList>('getAudioSources');
}

export async function saveRecorderScene(
  input: SaveRecorderSceneInput,
): Promise<RecorderScene> {
  return invoke<RecorderScene>('saveRecorderScene', { input });
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
