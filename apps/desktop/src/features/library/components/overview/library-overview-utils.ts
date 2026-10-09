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

export interface RecordingDateGroup {
  id: string;
  label: string;
  recordings: LibraryRecordingSummary[];
}

export function groupRecordingsByDate(
  recordings: LibraryRecordingSummary[],
  sortKey: LibraryRecordingSortKey,
  nowMs: number,
): RecordingDateGroup[] {
  if (sortKey !== 'started') {
    return [{ id: 'all', label: 'All recordings', recordings }];
  }

  const today = startOfLocalDay(nowMs);
  const yesterday = today - 86_400_000;
  const groups = new Map<string, RecordingDateGroup>();

  for (const recording of recordings) {
    const timestamp = recording.startedAtMs;
    const id = timestamp === null ? 'unknown-date' : localDateKey(timestamp);
    const label =
      timestamp === null
        ? 'Date unavailable'
        : recordingDateLabel(timestamp, today, yesterday);
    const group = groups.get(id);

    if (group) {
      group.recordings.push(recording);
    } else {
      groups.set(id, { id, label, recordings: [recording] });
    }
  }

  return [...groups.values()];
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
  return (
    recording.status === 'completed' ||
    recording.status === 'failed' ||
    recording.status === 'recovered-partial'
  );
}

export function isRecordingRenameable(
  recording: LibraryRecordingSummary,
): boolean {
  return (
    recording.status === 'completed' ||
    recording.status === 'failed' ||
    recording.status === 'recovered-partial'
  );
}

export function recordingTitle(recording: LibraryRecordingSummary): string {
  return recording.title ?? formatTimestamp(recording.startedAtMs);
}

export function uniqueIds(ids: string[]): string[] {
  return [...new Set(ids)];
}

export function keepThumbnailPaths<T extends string | null>(
  thumbnailPaths: Record<string, T>,
  recordings: LibraryRecordingSummary[],
): Record<string, T> {
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

function recordingDateLabel(
  timestampMs: number,
  todayMs: number,
  yesterdayMs: number,
): string {
  const day = startOfLocalDay(timestampMs);

  if (day === todayMs) {
    return 'Today';
  }
  if (day === yesterdayMs) {
    return 'Yesterday';
  }

  return new Intl.DateTimeFormat(undefined, {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  }).format(timestampMs);
}

function localDateKey(timestampMs: number): string {
  const date = new Date(timestampMs);
  return [
    date.getFullYear(),
    String(date.getMonth() + 1).padStart(2, '0'),
    String(date.getDate()).padStart(2, '0'),
  ].join('-');
}

function startOfLocalDay(timestampMs: number): number {
  const date = new Date(timestampMs);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}
