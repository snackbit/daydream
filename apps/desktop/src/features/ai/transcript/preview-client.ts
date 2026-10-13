import type {
  GetRecordingTranscriptPreviewInput,
  RecordingTranscriptPreview,
} from '@daydream/shared';
import { decodeResponse, invoke } from '@/lib/diagnostics/invoke';

export async function getRecordingTranscriptPreview(
  input: GetRecordingTranscriptPreviewInput,
): Promise<RecordingTranscriptPreview> {
  const value = await invoke<unknown>('getRecordingTranscriptPreview', {
    input,
  });
  return decodeResponse('getRecordingTranscriptPreview', value, (response) =>
    decodeTranscriptPreview(response, input),
  );
}

export function decodeTranscriptPreview(
  value: unknown,
  input: GetRecordingTranscriptPreviewInput,
): RecordingTranscriptPreview {
  if (
    !value ||
    typeof value !== 'object' ||
    !('recordingId' in value) ||
    value.recordingId !== input.recordingId ||
    !('sourceRole' in value) ||
    value.sourceRole !== input.sourceRole ||
    !('entries' in value) ||
    !Array.isArray(value.entries) ||
    value.entries.length > 3
  )
    throw new Error('Invalid transcript preview.');
  const entries = value.entries.map((entry: unknown) => {
    if (!entry || typeof entry !== 'object')
      throw new Error('Invalid transcript preview.');
    const row: Record<string, unknown> = Object.fromEntries(
      Object.entries(entry),
    );
    const { id, segmentId, startOffsetMs, endOffsetMs, text, truncated } = row;
    if (
      typeof id !== 'string' ||
      !id ||
      typeof segmentId !== 'string' ||
      !segmentId ||
      typeof startOffsetMs !== 'number' ||
      !Number.isSafeInteger(startOffsetMs) ||
      startOffsetMs < 0 ||
      typeof endOffsetMs !== 'number' ||
      !Number.isSafeInteger(endOffsetMs) ||
      endOffsetMs <= startOffsetMs ||
      startOffsetMs >= input.endOffsetMs ||
      endOffsetMs <= input.startOffsetMs ||
      typeof text !== 'string' ||
      Array.from(text).length > 240 ||
      typeof truncated !== 'boolean'
    )
      throw new Error('Invalid transcript preview.');
    return { id, segmentId, startOffsetMs, endOffsetMs, text, truncated };
  });
  if (
    new Set(entries.map((entry) => entry.id)).size !== entries.length ||
    entries.some(
      (entry, i) =>
        i > 0 && entry.startOffsetMs < (entries[i - 1]?.startOffsetMs ?? 0),
    )
  )
    throw new Error('Invalid transcript preview.');
  return {
    recordingId: input.recordingId,
    sourceRole: input.sourceRole,
    entries,
  };
}
