import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import type {
  DeleteLibraryRecordingInput,
  DeleteLibraryRecordingsInput,
  LibraryPlaybackAssetRequest,
  LibraryPlaybackAssets,
  LibraryPlaybackThumbnails,
  LibraryRecordingDetail,
  LibraryRecordingSummary,
  LibraryRecordingThumbnail,
  LibraryRecordingThumbnailRequest,
  LibraryWaveformRequest,
  LibraryWaveforms,
} from '@daydream/shared';

export async function listLibraryRecordings(): Promise<
  LibraryRecordingSummary[]
> {
  return invoke<LibraryRecordingSummary[]>('listLibraryRecordings');
}

export async function getLibraryRecording(
  recordingId: string,
): Promise<LibraryRecordingDetail> {
  return invoke<LibraryRecordingDetail>('getLibraryRecording', {
    recordingId,
  });
}

export async function deleteLibraryRecording(
  input: DeleteLibraryRecordingInput,
): Promise<void> {
  return invoke<void>('deleteLibraryRecording', { input });
}

export async function deleteLibraryRecordings(
  input: DeleteLibraryRecordingsInput,
): Promise<void> {
  return invoke<void>('deleteLibraryRecordings', { input });
}

export async function getRecordingWaveforms(
  input: LibraryWaveformRequest,
): Promise<LibraryWaveforms> {
  return invoke<LibraryWaveforms>('getRecordingWaveforms', { input });
}

export async function prepareLibraryPlaybackAssets(
  input: LibraryPlaybackAssetRequest,
): Promise<LibraryPlaybackAssets> {
  return invoke<LibraryPlaybackAssets>('prepareLibraryPlaybackAssets', {
    input,
  });
}

export async function prepareLibraryPlaybackThumbnails(
  input: LibraryPlaybackAssetRequest,
): Promise<LibraryPlaybackThumbnails> {
  return invoke<LibraryPlaybackThumbnails>('prepareLibraryPlaybackThumbnails', {
    input,
  });
}

export async function prepareLibraryRecordingThumbnail(
  input: LibraryRecordingThumbnailRequest,
): Promise<LibraryRecordingThumbnail> {
  return invoke<LibraryRecordingThumbnail>('prepareLibraryRecordingThumbnail', {
    input,
  });
}

export function localAssetUrl(path: string): string {
  return convertFileSrc(path);
}
