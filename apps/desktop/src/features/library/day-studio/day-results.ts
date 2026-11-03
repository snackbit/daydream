import type {
  RecordingActivity,
  RecordingAiProcessingState,
  RecordingTranscriptSegment,
  RecordingTranscriptionTrackState,
  RecordingTranscriptionProgress,
  VisualFrameProcessingState,
} from '@daydream/shared';
import type { StudioVisualFrame } from '../components/studio/observations/studio-visual-observations';
import type { StudioResultRange } from '../components/studio/results/studio-result-window-store';
import {
  dayPosition,
  projectSourceRange as range,
  type DayTimelineModel,
} from './day-timeline';

export interface DayRecordingResults {
  pending?: boolean;
  ai: RecordingAiProcessingState | null;
  activity: RecordingActivity | null;
  frames: StudioVisualFrame[];
  request?: StudioResultRange;
}
function transcript(
  model: DayTimelineModel,
  id: string,
  segment: RecordingTranscriptSegment,
): RecordingTranscriptSegment[] {
  return range(model, id, segment.startOffsetMs, segment.endOffsetMs).map(
    (r) => ({
      ...segment,
      startOffsetMs: r.start,
      endOffsetMs: r.end,
      words: segment.words
        .filter(
          (w) =>
            w.endOffsetMs > r.start - r.shift &&
            w.startOffsetMs < r.end - r.shift,
        )
        .map((w) => ({
          ...w,
          startOffsetMs: Math.max(r.start, w.startOffsetMs + r.shift),
          endOffsetMs: Math.min(r.end, w.endOffsetMs + r.shift),
        })),
    }),
  );
}
type DayProcessingView = RecordingAiProcessingState & {
  transcriptionTracks: RecordingTranscriptionTrackState[];
  transcriptionProgress: RecordingTranscriptionProgress[];
  visualFrames: VisualFrameProcessingState[];
};
export function projectDayResults(
  model: DayTimelineModel,
  results: Map<string, DayRecordingResults>,
  requestedRange?: StudioResultRange,
  failedIds: ReadonlySet<string> = new Set(),
) {
  const pendingRanges: { startOffsetMs: number; endOffsetMs: number }[] = [],
    failedRanges: typeof pendingRanges = [];
  if (requestedRange)
    for (const span of model.spans) {
      const startOffsetMs = Math.max(
        span.startMs - model.originMs,
        requestedRange.startOffsetMs,
      );
      const endOffsetMs = Math.min(
        span.startMs - model.originMs + span.durationMs,
        requestedRange.endOffsetMs,
      );
      if (endOffsetMs <= startOffsetMs) continue;
      const request = results.get(span.recordingId)?.request;
      if (
        !request ||
        results.get(span.recordingId)?.pending ||
        request.startOffsetMs !== requestedRange.startOffsetMs ||
        request.endOffsetMs !== requestedRange.endOffsetMs ||
        request.bucketMs !== requestedRange.bucketMs
      ) {
        (failedIds.has(span.recordingId) ? failedRanges : pendingRanges).push({
          startOffsetMs,
          endOffsetMs,
        });
      }
    }
  const ai: DayProcessingView = {
    recordingId: model.view.id,
    job: null,
    chunks: [],
    transcript: { recordingId: model.view.id, tracks: [] },
    transcriptionTracks: [],
    visualFrames: [],
    transcriptionProgress: [],
  };
  const activity: RecordingActivity = {
    recordingId: model.view.id,
    intervals: [],
    events: [],
    captureStates: [],
    windowSnapshots: [],
  };
  const frames: StudioVisualFrame[] = [];
  for (const [id, result] of results) {
    const ids = model.trackIds.get(id),
      track = (value: string) => ids?.get(value) ?? value;
    const state = result.ai;
    ai.resultsRevision = `${ai.resultsRevision ?? ''}:${id}:${state?.resultsRevision ?? ''}`;
    if (state?.timelineOverview !== undefined) {
      ai.timelineOverview ??= [];
      for (const bucket of state.timelineOverview)
        ai.timelineOverview.push(
          ...range(model, id, bucket.startOffsetMs, bucket.endOffsetMs).map(
            (r) => ({
              ...bucket,
              trackId: track(bucket.trackId),
              startOffsetMs: r.start,
              endOffsetMs: r.end,
            }),
          ),
        );
    }
    for (const t of state?.transcript.tracks ?? [])
      ai.transcript.tracks.push({
        ...t,
        trackId: track(t.trackId),
        segments: t.segments.flatMap((s) => transcript(model, id, s)),
      });
    for (const t of state?.transcriptionTracks ?? [])
      ai.transcriptionTracks.push({
        ...t,
        trackKey: `${id}:${t.trackKey}`,
        libraryTrackId: t.libraryTrackId ? track(t.libraryTrackId) : null,
        chunks: t.chunks.flatMap((c) =>
          range(model, id, c.startOffsetMs ?? 0, c.endOffsetMs ?? 0).map(
            (r) => ({
              ...c,
              startOffsetMs: r.start,
              endOffsetMs: r.end,
              transcriptSegments: c.transcriptSegments.flatMap((s) =>
                transcript(model, id, s),
              ),
            }),
          ),
        ),
      });
    for (const c of state?.chunks ?? [])
      ai.chunks.push(
        ...range(model, id, c.startOffsetMs ?? 0, c.endOffsetMs ?? 0).map(
          (r) => ({
            ...c,
            startOffsetMs: r.start,
            endOffsetMs: r.end,
            trackId: c.trackId ? track(c.trackId) : null,
          }),
        ),
      );
    for (const progress of state?.transcriptionProgress ?? [])
      ai.transcriptionProgress.push({
        ...progress,
        trackId: track(progress.trackId),
        ranges: progress.ranges.flatMap((r) =>
          range(model, id, r.startOffsetMs, r.endOffsetMs).map((r) => ({
            startOffsetMs: r.start,
            endOffsetMs: r.end,
          })),
        ),
        publishedRanges: progress.publishedRanges.flatMap((r) =>
          range(model, id, r.startOffsetMs, r.endOffsetMs).map((r) => ({
            startOffsetMs: r.start,
            endOffsetMs: r.end,
          })),
        ),
      });
    for (const f of state?.visualFrames ?? []) {
      const time = dayPosition(model, id, f.frameTimestampMs);
      if (time !== null) ai.visualFrames.push({ ...f, frameTimestampMs: time });
    }
    for (const i of result.activity?.intervals ?? [])
      activity.intervals.push(
        ...range(model, id, i.startOffsetMs, i.endOffsetMs).map((r) => ({
          ...i,
          startOffsetMs: r.start,
          endOffsetMs: r.end,
        })),
      );
    for (const e of result.activity?.events ?? []) {
      const time = dayPosition(model, id, e.offsetMs);
      if (time !== null) activity.events.push({ ...e, offsetMs: time });
    }
    for (const e of result.activity?.captureStates ?? []) {
      const time = dayPosition(model, id, e.offsetMs);
      if (time !== null) activity.captureStates.push({ ...e, offsetMs: time });
    }
    for (const e of result.activity?.windowSnapshots ?? []) {
      const time = dayPosition(model, id, e.offsetMs);
      if (time !== null)
        activity.windowSnapshots.push({ ...e, offsetMs: time });
    }
    for (const f of result.frames) {
      const time = dayPosition(model, id, f.frameTimestampMs);
      if (time === null) continue;
      const end =
        range(model, id, f.frameTimestampMs, f.endTimestampMs)[0]?.end ??
        time + 1;
      const sampleEnd =
        f.ocrSampleEndMs === undefined
          ? null
          : range(model, id, f.frameTimestampMs, f.ocrSampleEndMs)[0]?.end;
      frames.push({
        ...f,
        frameTimestampMs: time,
        endTimestampMs: end,
        sourceTrackId: track(f.sourceTrackId ?? ''),
        ...(sampleEnd == null ? {} : { ocrSampleEndMs: sampleEnd }),
      });
    }
  }
  return {
    ai,
    activity,
    pendingRanges,
    failedRanges,
    frames: frames.sort((a, b) => a.frameTimestampMs - b.frameTimestampMs),
  };
}
