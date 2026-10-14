import type { LibraryRecordingTrack } from '@daydream/shared';

export function orderedDetailTracks(
  tracks: LibraryRecordingTrack[],
  laneOrder: string[],
): LibraryRecordingTrack[] {
  const byId = new Map(tracks.map((track) => [track.id, track]));
  const ordered = laneOrder
    .map((trackId) => byId.get(trackId))
    .filter((track): track is LibraryRecordingTrack => Boolean(track));
  const orderedIds = new Set(ordered.map((track) => track.id));
  return [...ordered, ...tracks.filter((track) => !orderedIds.has(track.id))];
}
