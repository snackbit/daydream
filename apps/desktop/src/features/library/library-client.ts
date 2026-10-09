import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import type {
  CloseLibraryPlaybackSessionRequest,
  CreateLibraryRedactionsInput,
  DeleteLibraryRecordingInput,
  DeleteLibraryRecordingsInput,
  DeleteLibraryRedactionInput,
  LibraryRecordingDebugData,
  LibraryRecordingDebugInput,
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
  UnredactLibraryRedactionsInput,
  ResizeLibraryRedactionInput,
} from '@daydream/shared';
import {
  decodeVideoAssets,
  decodePlaybackRuns,
  decodeVideoStreams,
} from './video-metadata-decoder';

export async function resizeLibraryRedaction(
  input: ResizeLibraryRedactionInput,
): Promise<void> {
  await invoke<unknown>('resizeLibraryRedaction', { input });
}

export async function adjacentLibraryFrame(
  recordingId: string,
  timestampMs: number,
  direction: -1 | 1,
  sourceTrackId?: string,
): Promise<number | null> {
  const value = await invoke<unknown>('adjacentLibraryFrame', {
    input: {
      recordingId,
      timestampMs: Math.round(timestampMs),
      direction,
      ...(sourceTrackId ? { sourceTrackId } : {}),
    },
  });
  if (value === null) return null;
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0)
    throw new Error('Invalid frame navigation response.');
  return value;
}

export async function listLibraryRecordings(): Promise<
  LibraryRecordingSummary[]
> {
  return invoke<LibraryRecordingSummary[]>('listLibraryRecordings');
}

export async function getLibraryRecording(
  recordingId: string,
): Promise<LibraryRecordingDetail> {
  const detail = await invoke<LibraryRecordingDetail>('getLibraryRecording', {
    recordingId,
  });
  return {
    ...detail,
    segments: detail.segments.map((segment) => ({
      ...segment,
      ...(segment.videoStreams === undefined
        ? {}
        : { videoStreams: decodeVideoStreams(segment.videoStreams) }),
    })),
  };
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

export async function createLibraryRedactions(
  input: CreateLibraryRedactionsInput,
): Promise<LibraryRecordingDetail> {
  return invoke<LibraryRecordingDetail>('createLibraryRedactions', { input });
}

export async function deleteLibraryRedaction(
  input: DeleteLibraryRedactionInput,
): Promise<LibraryRecordingDetail> {
  return invoke<LibraryRecordingDetail>('deleteLibraryRedaction', { input });
}

export async function unredactLibraryRedactions(
  input: UnredactLibraryRedactionsInput,
): Promise<LibraryRecordingDetail> {
  return invoke<LibraryRecordingDetail>('unredactLibraryRedactions', {
    input,
  });
}

export async function getLibraryRecordingDebug(
  input: LibraryRecordingDebugInput,
): Promise<LibraryRecordingDebugData> {
  return invoke<LibraryRecordingDebugData>('getLibraryRecordingDebug', {
    input,
  });
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
  const assets = await invoke<LibraryPlaybackAssets>(
    'createLibraryPlaybackSession',
    {
      input,
    },
  );
  return {
    ...assets,
    ...(assets.runs === undefined
      ? {}
      : { runs: decodePlaybackRuns(assets.runs) }),
    segments: assets.segments.map((segment) => ({
      ...segment,
      ...(segment.videos === undefined
        ? {}
        : { videos: decodeVideoAssets(segment.videos) }),
    })),
  };
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
