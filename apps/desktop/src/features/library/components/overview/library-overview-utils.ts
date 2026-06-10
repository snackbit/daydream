import type {
  LibraryRecordingSortKey,
  LibraryRecordingSummary,
} from '@daydream/shared';
import { formatTimestamp } from '@/features/library/library-format';

export function sortRecordings(
  recordings: LibraryRecordingSummary[],
  sortKey: LibraryRecordingSortKey,
): LibraryRecordingSummary[] {
  return [...recordings].sort((left, right) => {
    switch (sortKey) {
      case 'availability':
        return left.availability.localeCompare(right.availability);
      case 'duration':
        return (right.durationMs ?? -1) - (left.durationMs ?? -1);
      case 'status':
        return left.status.localeCompare(right.status);
      case 'started':
        return (right.startedAtMs ?? -1) - (left.startedAtMs ?? -1);
    }
  });
}

export function selectedRange(
  recordings: LibraryRecordingSummary[],
  anchorId: string,
  recordingId: string,
): string[] {
  const anchorIndex = recordings.findIndex(
    (recording) => recording.id === anchorId,
  );
  const recordingIndex = recordings.findIndex(
    (recording) => recording.id === recordingId,
  );
  if (anchorIndex < 0 || recordingIndex < 0) {
    return [recordingId];
  }
  const [start, end] =
    anchorIndex < recordingIndex
      ? [anchorIndex, recordingIndex]
      : [recordingIndex, anchorIndex];

  return recordings
    .slice(start, end + 1)
    .filter(isRecordingSelectable)
    .map((recording) => recording.id);
}

export function isRecordingSelectable(
  recording: LibraryRecordingSummary,
): boolean {
  return recording.availability !== 'locked';
}

export function isRecordingDeletable(
  recording: LibraryRecordingSummary,
): boolean {
  return recording.status === 'completed' || recording.status === 'failed';
}

export function isRecordingRenameable(
  recording: LibraryRecordingSummary,
): boolean {
  return recording.status === 'completed' || recording.status === 'failed';
}

export function recordingTitle(recording: LibraryRecordingSummary): string {
  return recording.title ?? formatTimestamp(recording.startedAtMs);
}

export function uniqueIds(ids: string[]): string[] {
  return [...new Set(ids)];
}

export function keepThumbnailPaths(
  thumbnailPaths: Record<string, string>,
  recordings: LibraryRecordingSummary[],
): Record<string, string> {
  const recordingIds = new Set(recordings.map((recording) => recording.id));

  return Object.fromEntries(
    Object.entries(thumbnailPaths).filter(([recordingId]) =>
      recordingIds.has(recordingId),
    ),
  );
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
