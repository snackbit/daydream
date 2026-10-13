import type {
  GetRecordingTranscriptPreviewInput,
  RecordingTranscriptPreview,
} from '@daydream/shared';
import { getRecordingTranscriptPreview } from '@/features/ai/transcript/preview-client';
import type { DayTimelineModel } from '../day-timeline';

export async function readDayTranscriptPreview(
  model: DayTimelineModel,
  input: GetRecordingTranscriptPreviewInput,
): Promise<RecordingTranscriptPreview> {
  const entries: RecordingTranscriptPreview['entries'] = [];
  const seen = new Set<string>();
  for (const span of model.spans) {
    const start = span.startMs - model.originMs;
    const from = Math.max(start, input.startOffsetMs),
      to = Math.min(start + span.durationMs, input.endOffsetMs);
    if (to <= from) continue;
    const result = await getRecordingTranscriptPreview({
      recordingId: span.recordingId,
      sourceRole: input.sourceRole,
      startOffsetMs: Math.floor(span.sourceOffsetMs + from - start),
      endOffsetMs: Math.ceil(span.sourceOffsetMs + to - start),
    });
    for (const entry of result.entries) {
      const id = `${span.recordingId}:${entry.id}`;
      if (seen.has(id)) continue;
      seen.add(id);
      entries.push({
        ...entry,
        id,
        // The IPC decoder already proves overlap with this source read. Clip
        // to its playable span and translate directly to the day clock.
        startOffsetMs:
          start + Math.max(0, entry.startOffsetMs - span.sourceOffsetMs),
        endOffsetMs:
          start +
          Math.min(span.durationMs, entry.endOffsetMs - span.sourceOffsetMs),
      });
      if (entries.length === 3)
        return {
          recordingId: model.view.id,
          sourceRole: input.sourceRole,
          entries,
        };
    }
  }
  return { recordingId: model.view.id, sourceRole: input.sourceRole, entries };
}
