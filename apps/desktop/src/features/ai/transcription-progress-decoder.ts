import type {
  RecordingTranscriptionProgress,
  TranscriptSourceRole,
} from '@daydream/shared';
import {
  array,
  count,
  invalid,
  record,
  string,
  status,
} from '@/features/processing/task-decoder';

function sourceRole(value: unknown): TranscriptSourceRole {
  return value === 'microphone' || value === 'desktop-audio'
    ? value
    : invalid();
}
function range(value: unknown) {
  const row = record(value),
    startOffsetMs = count(row.startOffsetMs),
    endOffsetMs = count(row.endOffsetMs);
  if (endOffsetMs <= startOffsetMs) return invalid();
  return { startOffsetMs, endOffsetMs };
}
export function decodeTranscriptionProgress(
  value: unknown,
): RecordingTranscriptionProgress[] {
  return array(value, (value) => {
    const row = record(value);
    const ranges = array(row.ranges, range),
      publishedRanges = array(row.publishedRanges, range);
    const ordered = ranges.toSorted(
      (a, b) => a.startOffsetMs - b.startOffsetMs,
    );
    for (const published of publishedRanges) {
      let cursor = published.startOffsetMs;
      for (const requested of ordered) {
        if (requested.endOffsetMs <= cursor) continue;
        if (requested.startOffsetMs > cursor) break;
        cursor = Math.max(cursor, requested.endOffsetMs);
        if (cursor >= published.endOffsetMs) break;
      }
      if (cursor < published.endOffsetMs) return invalid();
    }
    return {
      jobId: string(row.jobId),
      trackId: string(row.trackId),
      sourceRole: sourceRole(row.sourceRole),
      status: status(row.status),
      ranges,
      publishedRanges,
    };
  });
}
