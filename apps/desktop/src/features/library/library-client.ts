import { convertFileSrc } from '@tauri-apps/api/core';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { LibraryMediaPrepared } from '@daydream/shared';
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
import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';
import { listen } from '@/lib/diagnostics/listen';
import { decodeAudioCleanupState } from './components/studio/audio-cleanup/audio-cleanup-client';
import {
  decodeVideoAssets,
  decodePlaybackRuns,
  decodeSeekCapability,
  decodeThumbnailBase,
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
  return decodeResponse('adjacentLibraryFrame', value, (value) => {
    if (value === null) return null;
    if (typeof value !== 'number' || !Number.isFinite(value) || value < 0)
      throw new Error('Invalid frame navigation response.');
    return value;
  });
}

export async function listLibraryRecordings(): Promise<
  LibraryRecordingSummary[]
> {
  return invoke<LibraryRecordingSummary[]>('listLibraryRecordings');
}

export async function getLibraryRecording(
  recordingId: string,
  options?: { includeDiskUsage: boolean },
): Promise<LibraryRecordingDetail> {
  const detail = await invoke<LibraryRecordingDetail>('getLibraryRecording', {
    recordingId,
    ...(options ? { includeDiskUsage: options.includeDiskUsage } : {}),
  });
  return decodeResponse('getLibraryRecording', detail, (detail) => ({
    ...detail,
    ...(detail.audioCleanup === undefined
      ? {}
      : { audioCleanup: decodeAudioCleanupState(detail.audioCleanup) }),
    segments: detail.segments.map((segment) => ({
      ...segment,
      ...(segment.videoStreams === undefined
        ? {}
        : { videoStreams: decodeVideoStreams(segment.videoStreams) }),
    })),
  }));
}

/** Reads the last measured disk usage without retaining private metadata. */
export async function getLibraryRecordingSize(
  recordingId: string,
): Promise<number> {
  return decodeResponse(
    'getLibraryRecording',
    await invoke<unknown>('getLibraryRecording', {
      recordingId,
      cachedDiskUsage: true,
    }),
    (value) => {
      if (
        typeof value !== 'object' ||
        value === null ||
        !('id' in value) ||
        value.id !== recordingId ||
        !('sizeBytes' in value) ||
        typeof value.sizeBytes !== 'number' ||
        !Number.isSafeInteger(value.sizeBytes) ||
        value.sizeBytes < 0
      ) {
        throw new Error('Recording disk usage is unavailable.');
      }
      return value.sizeBytes;
    },
  );
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
  return decodeResponse('createLibraryPlaybackSession', assets, (assets) => ({
    ...assets,
    ...(assets.runs === undefined
      ? {}
      : { runs: decodePlaybackRuns(assets.runs) }),
    segments: assets.segments.map((segment) => ({
      ...segment,
      ...decodeSeekCapability(segment.canSeekInStream),
      ...decodeThumbnailBase(segment.thumbnailBaseUrl),
      ...(segment.videos === undefined
        ? {}
        : { videos: decodeVideoAssets(segment.videos) }),
    })),
  }));
}

export async function closeLibraryPlaybackSession(
  input: CloseLibraryPlaybackSessionRequest,
): Promise<void> {
  return invoke<void>('closeLibraryPlaybackSession', { input });
}

export function listenLibraryMediaPrepared(
  handler: (event: LibraryMediaPrepared) => void,
): Promise<UnlistenFn> {
  return listen<unknown>('library-media-prepared', (event) => {
    const value = event.payload;
    if (
      typeof value !== 'object' ||
      value === null ||
      !('recordingId' in value) ||
      typeof value.recordingId !== 'string' ||
      value.recordingId.length === 0
    )
      throw new Error('Invalid playback preparation event');
    handler({ recordingId: value.recordingId });
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
