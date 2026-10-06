import { useMemo, useState } from 'react';
import type {
  LibraryRecordingDetail,
  LibraryPlaybackAssets,
} from '@daydream/shared';
import { useStudioTimelineStore } from './studio-timeline-store';

export function useStudioVideoSources(
  detail: LibraryRecordingDetail,
  assets: LibraryPlaybackAssets | null,
  inspectedSource?: string,
) {
  const tracks = useMemo(
    () => detail.tracks.filter((track) => track.kind === 'video'),
    [detail.tracks],
  );
  const [choice, setChoice] = useState<{
    recordingId: string;
    trackId: string;
  } | null>(null);
  const [gridRecord, setGridRecord] = useState<string | null>(null);
  const selectedTrackId =
    inspectedSource && tracks.some((track) => track.id === inspectedSource)
      ? inspectedSource
      : choice?.recordingId === detail.id &&
          tracks.some((track) => track.id === choice.trackId)
        ? choice.trackId
        : tracks[0]?.id;
  function select(trackId: string) {
    if (!tracks.some((track) => track.id === trackId)) return;
    const store = useStudioTimelineStore.getState();
    if (store.selection.visualFrameIds.length)
      store.setSelection({ ...store.selection, visualFrameIds: [] });
    setChoice({ recordingId: detail.id, trackId });
  }
  const selectedDetail = useMemo(
    () => ({
      ...detail,
      tracks: detail.tracks.filter(
        (track) => track.kind !== 'video' || track.id === selectedTrackId,
      ),
      segments: detail.segments.map((segment) => ({
        ...segment,
        videoStreamIndex:
          segment.videoStreams?.find(
            (stream) => stream.trackId === selectedTrackId,
          )?.streamIndex ?? segment.videoStreamIndex,
      })),
    }),
    [detail, selectedTrackId],
  );
  const selectedAssets = useMemo(
    () =>
      assets
        ? {
            ...assets,
            segments: assets.segments.map((segment) => {
              const video = segment.videos?.find(
                (video) => video.trackId === selectedTrackId,
              );
              return {
                ...segment,
                videoUrl:
                  video?.url ??
                  (selectedTrackId === tracks[0]?.id ? segment.videoUrl : ''),
                thumbnails:
                  video?.thumbnails ??
                  (selectedTrackId === tracks[0]?.id ? segment.thumbnails : []),
              };
            }),
          }
        : null,
    [assets, selectedTrackId, tracks],
  );
  return {
    tracks,
    selectedTrackId,
    selectedDetail,
    selectedAssets,
    grid: gridRecord === detail.id,
    select,
    toggleGrid: () =>
      setGridRecord((current) => (current === detail.id ? null : detail.id)),
  };
}
export type StudioVideoSources = ReturnType<typeof useStudioVideoSources>;
