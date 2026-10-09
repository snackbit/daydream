import type { LibraryRecordingDetail } from '@daydream/shared';
import { formatTimestamp } from '../library-format';

export const RECORDING_TITLE_MAX_CHARS = 120;

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function isRecordingDeletable(detail: LibraryRecordingDetail): boolean {
  return (
    detail.status === 'completed' ||
    detail.status === 'failed' ||
    detail.status === 'recovered-partial'
  );
}

export function isRecordingRenameable(detail: LibraryRecordingDetail): boolean {
  return (
    detail.status === 'completed' ||
    detail.status === 'failed' ||
    detail.status === 'recovered-partial'
  );
}

export function isAiProcessingStartable(
  detail: LibraryRecordingDetail,
): boolean {
  return (
    (detail.status === 'completed' || detail.status === 'recovered-partial') &&
    detail.availability === 'available'
  );
}

export function recordingTitle(detail: LibraryRecordingDetail): string {
  return detail.title ?? formatTimestamp(detail.startedAtMs);
}
