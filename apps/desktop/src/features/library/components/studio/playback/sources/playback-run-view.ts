import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryRecordingSegment,
} from '@daydream/shared';

/** Run assets own grouping; original detail stays available for citations and technical inspection. */
export function playbackRunView(
  detail: LibraryRecordingDetail,
  assets: LibraryPlaybackAssets | null,
) {
  if (!assets?.runs?.length) return { detail, assets };
  const segments = new Map(
    detail.segments.map((segment) => [segment.id, segment]),
  );
  const consumed = new Set<string>();
  const combined: LibraryRecordingSegment[] = [];
  for (const run of assets.runs) {
    const first = segments.get(run.segmentIds[0] ?? ''),
      last = segments.get(run.segmentIds.at(-1) ?? '');
    if (!first || !last || run.segmentIds.some((id) => !segments.has(id)))
      continue;
    for (const id of run.segmentIds) consumed.add(id);
    combined.push({
      ...first,
      id: run.runId,
      durationMs: run.durationMs,
      playbackOffsetMs: run.playbackOffsetMs,
      endedAtMs: last.endedAtMs,
      pauseAfterMs: last.pauseAfterMs,
      fragmentIds: run.segmentIds,
    });
  }
  combined.push(
    ...detail.segments.filter((segment) => !consumed.has(segment.id)),
  );
  combined.sort(
    (a, b) =>
      a.playbackOffsetMs - b.playbackOffsetMs ||
      a.segmentIndex - b.segmentIndex,
  );
  return {
    detail: { ...detail, segments: combined },
    assets: {
      ...assets,
      segments: [
        ...assets.runs.map((run) => run.assets),
        ...assets.segments.filter(
          (segment) => !consumed.has(segment.segmentId),
        ),
      ],
    },
  };
}
