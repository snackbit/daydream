import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import type {
  CloseLibraryPlaybackSessionRequest,
  DeleteLibraryRecordingInput,
  DeleteLibraryRecordingsInput,
  LibraryPlaybackAssetRequest,
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryRecordingSummary,
  LibraryRecordingThumbnail,
  LibraryRecordingThumbnailRequest,
  LibraryWaveformTileRequest,
  LibraryWaveformTiles,
  LibraryWaveformRequest,
  LibraryWaveforms,
  RenameLibraryRecordingInput,
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

export async function renameLibraryRecording(
  input: RenameLibraryRecordingInput,
): Promise<LibraryRecordingDetail> {
  return invoke<LibraryRecordingDetail>('renameLibraryRecording', { input });
}

export async function getRecordingWaveforms(
  input: LibraryWaveformRequest,
): Promise<LibraryWaveforms> {
  return invoke<LibraryWaveforms>('getRecordingWaveforms', { input });
}

export async function getRecordingWaveformTiles(
  input: LibraryWaveformTileRequest,
): Promise<LibraryWaveformTiles> {
  return invoke<LibraryWaveformTiles>('getRecordingWaveformTiles', { input });
}

export async function createLibraryPlaybackSession(
  input: LibraryPlaybackAssetRequest,
): Promise<LibraryPlaybackAssets> {
  return invoke<LibraryPlaybackAssets>('createLibraryPlaybackSession', {
    input,
  });
}

export async function closeLibraryPlaybackSession(
  input: CloseLibraryPlaybackSessionRequest,
): Promise<void> {
  return invoke<void>('closeLibraryPlaybackSession', { input });
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
