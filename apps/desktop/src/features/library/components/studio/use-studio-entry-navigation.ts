import { useCallback, useEffect, useRef, useState } from 'react';
import type { InspectorContentEntry } from './inspector/content/inspector-content';
import type { StudioVideoSources } from './use-studio-video-sources';

export function useStudioEntryNavigation(
  recordingId: string,
  privacyRevision: string,
  sources: StudioVideoSources,
  playbackReady: boolean,
  seek: (time: number) => void,
) {
  const [pending, setPending] = useState<{
    recordingId: string;
    privacyRevision: string;
    entry: InspectorContentEntry;
  } | null>(null);
  const applied = useRef<typeof pending>(null);
  useEffect(() => {
    if (!pending || applied.current === pending) return;
    if (
      pending.recordingId !== recordingId ||
      pending.privacyRevision !== privacyRevision ||
      !sources.tracks.some((track) => track.id === pending.entry.trackId)
    ) {
      applied.current = pending;
      return;
    }
    if (!playbackReady || sources.selectedTrackId !== pending.entry.trackId)
      return;
    applied.current = pending;
    seek(pending.entry.startMs);
  }, [
    pending,
    recordingId,
    privacyRevision,
    playbackReady,
    sources.selectedTrackId,
    sources.tracks,
    seek,
  ]);
  return useCallback(
    (entry: InspectorContentEntry) => {
      if (
        entry.kind === 'observation' &&
        sources.tracks.some((track) => track.id === entry.trackId) &&
        sources.selectedTrackId !== entry.trackId
      ) {
        setPending({ recordingId, privacyRevision, entry });
        sources.select(entry.trackId);
      } else {
        setPending(null);
        seek(entry.startMs);
      }
    },
    [recordingId, privacyRevision, sources, seek],
  );
}
