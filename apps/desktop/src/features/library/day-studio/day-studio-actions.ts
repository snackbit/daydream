import type {
  LibraryWaveformTileRequest,
  LibraryWaveformTiles,
  ResizeLibraryRedactionInput,
} from '@daydream/shared';
import { getRecordingWaveformTiles } from '../library-client';
import type { DayTimelineModel } from './day-timeline';

export function sourceRedaction(model: DayTimelineModel, id: string) {
  const view = model.view.redactions.find((c) => c.id === id);
  const recording = model.manifest.recordings.find(
    (r) => r.id === view?.recordingId,
  );
  const cut = recording?.redactions.find((c) => id.startsWith(`${c.id}:`));
  if (!view || !recording || !cut)
    throw new Error('The redaction is no longer available. Refresh the day.');
  return { view, recording, cut };
}
export function prepareDayRedactionResize(
  model: DayTimelineModel,
  input: ResizeLibraryRedactionInput,
) {
  const { view, recording, cut } = sourceRedaction(model, input.redactionId);
  const span = model.spans.find(
    (s) =>
      s.recordingId === recording.id &&
      input.redactionId === `${cut.id}:${s.segmentId}`,
  );
  if (!span) throw new Error('The recording is no longer available.');
  const start = span.startMs - model.originMs,
    shift = span.sourceOffsetMs - start;
  if (
    input.startOffsetMs < start ||
    input.endOffsetMs > start + span.durationMs
  )
    throw new Error('Keep the redaction inside its recording segment.');
  if (
    (input.startOffsetMs !== view.startOffsetMs &&
      view.startOffsetMs + shift !== cut.startOffsetMs) ||
    (input.endOffsetMs !== view.endOffsetMs &&
      view.endOffsetMs + shift !== cut.endOffsetMs)
  )
    throw new Error('Select the redaction’s original boundary to resize it.');
  return {
    ...input,
    recordingId: recording.id,
    redactionId: cut.id,
    startOffsetMs:
      input.startOffsetMs === view.startOffsetMs
        ? cut.startOffsetMs
        : input.startOffsetMs + shift,
    endOffsetMs:
      input.endOffsetMs === view.endOffsetMs
        ? cut.endOffsetMs
        : input.endOffsetMs + shift,
    expectedStartOffsetMs: cut.startOffsetMs,
    expectedEndOffsetMs: cut.endOffsetMs,
  };
}
export async function readDayWaveformTiles(
  model: DayTimelineModel,
  input: LibraryWaveformTileRequest,
): Promise<LibraryWaveformTiles> {
  const output: LibraryWaveformTiles = {
    recordingId: model.view.id,
    binMs: 0,
    error: null,
    pending: false,
    tracks: [],
  };
  const groups = new Map<string, typeof model.spans>();
  for (const span of model.spans) {
    const start = span.startMs - model.originMs;
    if (
      start >= input.endTimelineMs ||
      start + span.durationMs <= input.startTimelineMs
    )
      continue;
    const spans = groups.get(span.recordingId) ?? [];
    spans.push(span);
    groups.set(span.recordingId, spans);
  }
  const entries = [...groups];
  let index = 0;
  async function worker() {
    while (index < entries.length) {
      const entry = entries[index++];
      if (!entry) return;
      const [recordingId, spans] = entry;
      const ranges = spans.map((span) => {
        const start = span.startMs - model.originMs;
        return {
          from:
            span.sourceOffsetMs +
            Math.max(start, input.startTimelineMs) -
            start,
          to:
            span.sourceOffsetMs +
            Math.min(start + span.durationMs, input.endTimelineMs) -
            start,
        };
      });
      const from = Math.min(...ranges.map((range) => range.from));
      const to = Math.max(...ranges.map((range) => range.to));
      const result = await getRecordingWaveformTiles({
        ...input,
        recordingId,
        startTimelineMs: Math.round(from),
        endTimelineMs: Math.round(to),
        viewportWidthPx: Math.max(
          1,
          (input.viewportWidthPx * (to - from)) /
            Math.max(1, input.endTimelineMs - input.startTimelineMs),
        ),
      });
      output.binMs = result.binMs;
      output.error = result.error ?? output.error;
      output.pending ||= result.pending === true;
      for (const track of result.tracks) {
        const id =
          track.kind === 'desktop-audio' ? 'desktop-audio-1' : 'microphone-2';
        let target = output.tracks.find(
          (candidate) => candidate.trackId === id,
        );
        if (!target) {
          target = { ...track, trackId: id, tiles: [] };
          output.tracks.push(target);
        }
        target.scaleAmplitude = Math.max(
          target.scaleAmplitude,
          track.scaleAmplitude,
        );
        for (const span of spans) {
          const start = span.startMs - model.originMs;
          target.tiles.push(
            ...track.tiles
              .filter((tile) => tile.segmentId === span.segmentId)
              .map((tile) => ({
                ...tile,
                timelineStartMs:
                  tile.timelineStartMs + start - span.sourceOffsetMs,
              })),
          );
        }
      }
    }
  }
  await Promise.all([worker(), worker()]);
  return output;
}
