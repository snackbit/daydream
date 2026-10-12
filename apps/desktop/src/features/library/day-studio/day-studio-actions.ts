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
    tracks: [],
  };
  for (const span of model.spans) {
    const start = span.startMs - model.originMs,
      from = Math.max(start, input.startTimelineMs),
      to = Math.min(start + span.durationMs, input.endTimelineMs);
    if (to <= from) continue;
    const result = await getRecordingWaveformTiles({
      ...input,
      recordingId: span.recordingId,
      startTimelineMs: Math.round(span.sourceOffsetMs + from - start),
      endTimelineMs: Math.round(span.sourceOffsetMs + to - start),
      viewportWidthPx: Math.max(
        1,
        (input.viewportWidthPx * (to - from)) /
          Math.max(1, input.endTimelineMs - input.startTimelineMs),
      ),
    });
    output.binMs = result.binMs;
    output.error = result.error ?? output.error;
    for (const track of result.tracks) {
      const id =
        track.kind === 'desktop-audio' ? 'desktop-audio-1' : 'microphone-2';
      let target = output.tracks.find((t) => t.trackId === id);
      if (!target) {
        target = { ...track, trackId: id, tiles: [] };
        output.tracks.push(target);
      }
      target.scaleAmplitude = Math.max(
        target.scaleAmplitude,
        track.scaleAmplitude,
      );
      target.tiles.push(
        ...track.tiles
          .filter((t) => t.segmentId === span.segmentId)
          .map((t) => ({
            ...t,
            timelineStartMs: t.timelineStartMs + start - span.sourceOffsetMs,
          })),
      );
    }
  }
  return output;
}
