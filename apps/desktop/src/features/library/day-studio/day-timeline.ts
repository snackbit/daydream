import type {
  CreateLibraryRedactionRangeInput,
  LibraryDayTimeline,
  LibraryDayPlaybackSpan,
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryRecordingSegment,
  LibraryRecordingTrack,
} from '@daydream/shared';
import { STUDIO_ACTIVITY_LANES } from '../components/studio/timeline/lanes/activity/studio-activity-lanes';

export interface DayTimelineModel {
  manifest: LibraryDayTimeline;
  originMs: number;
  /** Render adapter only. Its ID must never be passed to recording IPC. */
  view: LibraryRecordingDetail;
  spans: LibraryDayPlaybackSpan[];
  sourceSpans: Map<string, LibraryDayPlaybackSpan[]>;
  trackIds: Map<string, Map<string, string>>;
}

export function buildDayTimeline(
  manifest: LibraryDayTimeline,
): DayTimelineModel {
  const originMs = manifest.spans[0]?.startMs ?? manifest.startMs;
  const recordings = new Map(
    manifest.recordings.map((recording) => [
      recording.id,
      {
        recording,
        segments: new Map(
          recording.segments.map((segment) => [segment.id, segment]),
        ),
      },
    ]),
  );
  const trackIds = new Map<string, Map<string, string>>(),
    tracks = new Map<string, LibraryRecordingTrack>();
  for (const recording of manifest.recordings) {
    const sourceIds = new Map(
      recording.segments.flatMap((segment) =>
        (segment.videoStreams ?? []).map(
          (stream) => [stream.trackId, stream.sourceId] as const,
        ),
      ),
    );
    const videoCount = recording.tracks.filter(
      (track) => track.kind === 'video',
    ).length;
    const ids = new Map<string, string>();
    for (const track of recording.tracks) {
      const sourceId = sourceIds.get(track.id);
      const identity = sourceId
        ? `source-${encodeURIComponent(sourceId)}`
        : videoCount === 1
          ? 'legacy-composite'
          : `recording-${recording.id}-${track.id}`;
      const id =
        track.kind === 'video'
          ? `day-video-${identity}`
          : track.kind === 'desktop-audio'
            ? 'desktop-audio-1'
            : 'microphone-2';
      ids.set(track.id, id);
      if (!tracks.has(id)) tracks.set(id, { ...track, id });
    }
    for (const lane of STUDIO_ACTIVITY_LANES) ids.set(lane.id, lane.id);
    trackIds.set(recording.id, ids);
  }
  const segments: LibraryRecordingSegment[] = [];
  const spans: LibraryDayPlaybackSpan[] = [];
  // Overlapping historical captures use a stable first-recorded owner. Trimming
  // keeps the transport monotonic without losing the original source offset.
  let previousEnd = originMs;
  for (const original of manifest.spans) {
    const trim = Math.max(0, previousEnd - original.startMs);
    if (trim >= original.durationMs) continue;
    const span = {
      ...original,
      startMs: original.startMs + trim,
      durationMs: original.durationMs - trim,
      sourceStartMs: original.sourceStartMs + trim,
      sourceOffsetMs: original.sourceOffsetMs + trim,
    };
    const source = recordings.get(span.recordingId);
    const recording = source?.recording;
    const segment = source?.segments.get(span.segmentId);
    const ids = trackIds.get(span.recordingId);
    if (!recording || !segment || !ids) continue;
    const videos = segment.videoStreams?.length
      ? segment.videoStreams
      : recording.tracks
          .filter((t) => t.kind === 'video')
          .map((t) => ({
            trackId: t.id,
            sourceId: null,
            label: t.label,
            streamIndex: segment.videoStreamIndex ?? 0,
            width: null,
            height: null,
            fps: null,
          }));
    segments.push({
      ...segment,
      mediaStartMs: span.sourceStartMs,
      startedAtMs: span.startMs,
      endedAtMs: span.startMs + span.durationMs,
      durationMs: span.durationMs,
      playbackOffsetMs: span.startMs - originMs,
      pauseAfterMs: 0,
      videoStreams: videos.map((v) => ({
        ...v,
        trackId: ids.get(v.trackId) ?? v.trackId,
      })),
      fileExists: true,
      state: 'finalized',
    });
    spans.push(span);
    previousEnd = span.startMs + span.durationMs;
  }
  for (const [i, segment] of segments.entries())
    segment.pauseAfterMs = Math.max(
      0,
      (segments[i + 1]?.playbackOffsetMs ??
        segment.playbackOffsetMs + (segment.durationMs ?? 0)) -
        segment.playbackOffsetMs -
        (segment.durationMs ?? 0),
    );
  const redactions = manifest.recordings.flatMap((recording) =>
    spans
      .filter((s) => s.recordingId === recording.id)
      .flatMap((span) =>
        recording.redactions.flatMap((cut) => {
          const from = Math.max(cut.startOffsetMs, span.sourceOffsetMs),
            to = Math.min(
              cut.endOffsetMs,
              span.sourceOffsetMs + span.durationMs,
            );
          return to > from
            ? [
                {
                  ...cut,
                  id: `${cut.id}:${span.segmentId}`,
                  startOffsetMs:
                    span.startMs - originMs + from - span.sourceOffsetMs,
                  endOffsetMs:
                    span.startMs - originMs + to - span.sourceOffsetMs,
                  trackIds: cut.trackIds.map(
                    (id) => trackIds.get(recording.id)?.get(id) ?? id,
                  ),
                },
              ]
            : [];
        }),
      ),
  );
  const view: LibraryRecordingDetail = {
    id: `day:${manifest.dayId}`,
    availability: 'available',
    durationMs: spans.reduce((sum, s) => sum + s.durationMs, 0),
    endedAtMs: previousEnd,
    startedAtMs: originMs,
    segments,
    tracks: [...tracks.values()],
    redactions,
    playableSegmentCount: segments.length,
    missingReasons: [],
    sizeBytes: manifest.recordings.reduce<number | null>(
      (sum, r) =>
        sum === null || r.sizeBytes === null ? null : sum + r.sizeBytes,
      0,
    ),
    status: 'completed',
    storageDir: '',
    thumbnailPath: null,
    title: manifest.dayId,
  };
  const sourceSpans = new Map<string, LibraryDayPlaybackSpan[]>();
  for (const span of spans) {
    const source = sourceSpans.get(span.recordingId) ?? [];
    source.push(span);
    sourceSpans.set(span.recordingId, source);
  }
  for (const source of sourceSpans.values())
    source.sort((a, b) => a.sourceOffsetMs - b.sourceOffsetMs);
  return { manifest, originMs, view, spans, trackIds, sourceSpans };
}

function firstSourceSpanAfter(
  spans: readonly LibraryDayPlaybackSpan[],
  offset: number,
) {
  let left = 0,
    right = spans.length;
  while (left < right) {
    const middle = (left + right) >>> 1;
    const span = spans[middle];
    if (span && span.sourceOffsetMs + span.durationMs <= offset)
      left = middle + 1;
    else right = middle;
  }
  return left;
}

/** Source spans are disjoint after the day adapter applies capture ownership. */
export function projectSourceRange(
  model: DayTimelineModel,
  id: string,
  from: number,
  to: number,
) {
  const spans = model.sourceSpans.get(id) ?? [];
  const output: { start: number; end: number; shift: number }[] = [];
  for (
    let index = firstSourceSpanAfter(spans, from);
    index < spans.length;
    index++
  ) {
    const span = spans[index];
    if (!span || span.sourceOffsetMs >= to) break;
    const start = Math.max(from, span.sourceOffsetMs),
      end = Math.min(to, span.sourceOffsetMs + span.durationMs);
    const shift = span.startMs - model.originMs - span.sourceOffsetMs;
    if (end > start) {
      const previous = output.at(-1);
      if (
        previous &&
        previous.shift === shift &&
        previous.end === start + shift
      )
        previous.end = end + shift;
      else output.push({ start: start + shift, end: end + shift, shift });
    }
  }
  return output;
}

export function dayPosition(
  model: DayTimelineModel,
  recordingId: string,
  offsetMs: number,
): number | null {
  const spans = model.sourceSpans.get(recordingId) ?? [];
  const candidate = spans[firstSourceSpanAfter(spans, offsetMs)];
  const span =
    candidate && offsetMs >= candidate.sourceOffsetMs ? candidate : undefined;
  return span
    ? span.startMs - model.originMs + offsetMs - span.sourceOffsetMs
    : null;
}
export function sourcePosition(
  model: DayTimelineModel,
  dayMs: number,
): { span: LibraryDayPlaybackSpan; offsetMs: number } | null {
  const span = model.spans.find(
    (s) =>
      dayMs >= s.startMs - model.originMs &&
      dayMs < s.startMs - model.originMs + s.durationMs,
  );
  return span
    ? {
        span,
        offsetMs: span.sourceOffsetMs + dayMs - (span.startMs - model.originMs),
      }
    : null;
}
export function dayPlaybackAssets(
  model: DayTimelineModel,
  sessions: Map<string, LibraryPlaybackAssets>,
): LibraryPlaybackAssets {
  const recordings = new Map(
    model.manifest.recordings.map((recording) => [recording.id, recording]),
  );
  const assets = new Map(
    [...sessions].map(([id, session]) => [
      id,
      new Map(session.segments.map((segment) => [segment.segmentId, segment])),
    ]),
  );
  return {
    recordingId: model.view.id,
    canSeekInStream: [...sessions.values()].every(
      (session) => session.canSeekInStream === true,
    ),
    sessionId: '',
    error: null,
    segments: model.spans.flatMap((span) => {
      const asset = assets.get(span.recordingId)?.get(span.segmentId);
      const ids = model.trackIds.get(span.recordingId);
      const recording = recordings.get(span.recordingId);
      const thumbnails = (
        values: LibraryPlaybackAssets['segments'][number]['thumbnails'],
        trackId: string | undefined,
      ) =>
        values
          .filter((thumb) => {
            const sourceTime =
              span.sourceOffsetMs + thumb.offsetMs - span.sourceStartMs;
            return (
              trackId !== undefined &&
              thumb.offsetMs >= span.sourceStartMs &&
              thumb.offsetMs < span.sourceStartMs + span.durationMs &&
              !recording?.redactions.some(
                (cut) =>
                  sourceTime >= cut.startOffsetMs &&
                  sourceTime < cut.endOffsetMs &&
                  (cut.trackIds.length === 0 || cut.trackIds.includes(trackId)),
              )
            );
          })
          .map((thumb) => ({
            ...thumb,
            offsetMs: thumb.offsetMs - span.sourceStartMs,
          }));
      return asset && ids
        ? [
            {
              ...asset,
              thumbnails: thumbnails(
                asset.thumbnails,
                recording?.tracks.find((track) => track.kind === 'video')?.id,
              ),
              audio: asset.audio.map((a) => ({
                ...a,
                trackId: ids.get(a.trackId) ?? a.trackId,
              })),
              ...(asset.videos
                ? {
                    videos: asset.videos.map((v) => ({
                      ...v,
                      trackId: ids.get(v.trackId) ?? v.trackId,
                      thumbnails: thumbnails(v.thumbnails, v.trackId),
                    })),
                  }
                : {}),
            },
          ]
        : [];
    }),
  };
}
export function splitDayRanges(
  model: DayTimelineModel,
  ranges: CreateLibraryRedactionRangeInput[],
): Map<string, CreateLibraryRedactionRangeInput[]> {
  const output = new Map<string, CreateLibraryRedactionRangeInput[]>();
  for (const span of model.spans)
    for (const range of ranges) {
      const start = span.startMs - model.originMs,
        from = Math.max(start, range.startOffsetMs),
        to = Math.min(start + span.durationMs, range.endOffsetMs);
      const ids = model.trackIds.get(span.recordingId);
      const trackIds = [...(ids ?? [])]
        .filter(([, id]) => range.trackIds.includes(id))
        .map(([id]) => id);
      if (to <= from || !trackIds.length) continue;
      const local = {
        ...range,
        startOffsetMs: Math.round(span.sourceOffsetMs + from - start),
        endOffsetMs: Math.round(span.sourceOffsetMs + to - start),
        trackIds,
      };
      output.set(span.recordingId, [
        ...(output.get(span.recordingId) ?? []),
        local,
      ]);
    }
  return output;
}
