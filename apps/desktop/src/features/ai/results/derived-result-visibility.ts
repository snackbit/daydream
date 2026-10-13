import type {
  LibraryRecordingDetail,
  RecordingActivity,
  RecordingAiProcessingState,
  RecordingVisualObservations,
  TranscriptSourceRole,
} from '@daydream/shared';

// Native transcript UUIDs and Library lane IDs identify the same source through
// its role. Never compare an opaque persisted UUID to a synthetic lane ID alone.
function audioTrackIds(
  detail: LibraryRecordingDetail,
  role: TranscriptSourceRole,
  id: string | null,
): string[] {
  return [
    ...detail.tracks
      .filter((track) => track.kind === role)
      .map((track) => track.id),
    ...(id ? [id] : []),
  ];
}

function overlaps(
  detail: LibraryRecordingDetail,
  ids: string[],
  start: number | null,
  end: number | null,
): boolean {
  return detail.redactions.some(
    (redaction) =>
      (redaction.trackIds.length === 0 ||
        redaction.trackIds.some((id) => ids.includes(id))) &&
      (start === null ||
        end === null ||
        (start < redaction.endOffsetMs && end > redaction.startOffsetMs)),
  );
}

function contains(
  detail: LibraryRecordingDetail,
  ids: string[],
  timestamp: number,
): boolean {
  return overlaps(detail, ids, timestamp, timestamp + 1);
}

export function visibleAiProcessingState(
  detail: LibraryRecordingDetail,
  state: RecordingAiProcessingState | null,
): RecordingAiProcessingState | null {
  if (!state || state.recordingId !== detail.id) return null;
  if (detail.redactions.length === 0) return state;
  const videoIds = detail.tracks
    .filter((track) => track.kind === 'video')
    .map((track) => track.id);
  return {
    ...state,
    ...(state.timelineOverview === undefined
      ? {}
      : {
          timelineOverview: state.timelineOverview.filter(
            (bucket) =>
              !overlaps(
                detail,
                [bucket.trackId],
                bucket.startOffsetMs,
                bucket.endOffsetMs,
              ),
          ),
        }),
    transcript: {
      ...state.transcript,
      tracks: state.transcript.tracks.map((track) => ({
        ...track,
        segments: track.segments.filter(
          (segment) =>
            !overlaps(
              detail,
              audioTrackIds(detail, track.sourceRole, track.trackId),
              segment.startOffsetMs,
              segment.endOffsetMs,
            ),
        ),
      })),
    },
    ...(state.transcriptionTracks
      ? {
          transcriptionTracks: state.transcriptionTracks.map((track) => {
            const ids = audioTrackIds(
              detail,
              track.sourceRole,
              track.libraryTrackId,
            );
            const chunks = track.chunks.map((chunk) => ({
              ...chunk,
              text: overlaps(
                detail,
                ids,
                chunk.startOffsetMs,
                chunk.endOffsetMs,
              )
                ? null
                : chunk.text,
              transcriptSegments: chunk.transcriptSegments.filter(
                (segment) =>
                  !overlaps(
                    detail,
                    ids,
                    segment.startOffsetMs,
                    segment.endOffsetMs,
                  ),
              ),
            }));
            return {
              ...track,
              chunks,
              textSegmentCount: chunks.reduce(
                (count, chunk) => count + chunk.transcriptSegments.length,
                0,
              ),
            };
          }),
        }
      : {}),
    ...(state.visualFrames
      ? {
          visualFrames: state.visualFrames.filter(
            (frame) => !contains(detail, videoIds, frame.frameTimestampMs),
          ),
        }
      : {}),
  };
}

export function visibleVisualObservations(
  detail: LibraryRecordingDetail,
  state: RecordingVisualObservations | null,
): RecordingVisualObservations | null {
  if (!state || state.recordingId !== detail.id) return null;
  if (detail.redactions.length === 0) return state;
  const ids = detail.tracks
    .filter((track) => track.kind === 'video')
    .map((track) => track.id);
  return {
    ...state,
    ocrRegions: state.ocrRegions.filter(
      (region) => !contains(detail, ids, region.frameTimestampMs),
    ),
    vlmObservations: state.vlmObservations.filter(
      (observation) =>
        !overlaps(
          detail,
          ids,
          observation.contextStartMs ?? observation.frameTimestampMs,
          observation.confirmedEndMs ?? observation.frameTimestampMs + 1,
        ),
    ),
  };
}

export function visibleActivity(
  detail: LibraryRecordingDetail,
  state: RecordingActivity | null,
): RecordingActivity | null {
  if (!state || state.recordingId !== detail.id) return null;
  if (detail.redactions.length === 0) return state;
  const focusIds = ['activity:focus'];
  return {
    ...state,
    events: state.events.filter(
      (event) => !contains(detail, focusIds, event.offsetMs),
    ),
    windowSnapshots: state.windowSnapshots.filter(
      (snapshot) => !contains(detail, focusIds, snapshot.offsetMs),
    ),
    intervals: state.intervals.filter(
      (interval) =>
        !overlaps(
          detail,
          [`activity:${interval.sourceKind}`],
          interval.startOffsetMs,
          interval.endOffsetMs,
        ),
    ),
    captureStates: state.captureStates.filter(
      (capture) =>
        !contains(
          detail,
          ['activity:mouse', 'activity:keyboard'],
          capture.offsetMs,
        ),
    ),
  };
}
