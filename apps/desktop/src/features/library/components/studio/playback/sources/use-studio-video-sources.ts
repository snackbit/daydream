import { useMemo, useState } from 'react';
import type {
  LibraryRecordingDetail,
  LibraryPlaybackAssets,
} from '@daydream/shared';
import { useStudioTimelineStore } from '../../timeline/state/studio-timeline-store';

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
  const [gridChoice, setGridChoice] = useState<{
    recordingId: string;
    grid: boolean;
  } | null>(null);
  const grid = inspectedSource
    ? false
    : gridChoice?.recordingId === detail.id
      ? gridChoice.grid
      : tracks.length > 1;
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
    setGridChoice({ recordingId: detail.id, grid: false });
  }
  const selectedDetail = useMemo(
    () => ({
      ...detail,
      tracks: detail.tracks.filter(
        (track) => track.kind !== 'video' || track.id === selectedTrackId,
      ),
      segments: detail.segments.map((segment) => {
        const stream =
          segment.videoStreams?.find(
            (stream) => stream.trackId === selectedTrackId,
          ) ?? (grid ? segment.videoStreams?.[0] : undefined);
        return {
          ...segment,
          // Source absence belongs to the asset URL; retain this segment's timing in the transport.
          videoStreamIndex: stream?.streamIndex ?? segment.videoStreamIndex,
          videoTrackId: stream?.trackId ?? selectedTrackId,
        };
      }),
    }),
    [detail, selectedTrackId, grid],
  );
  const selectedAssets = useMemo(
    () =>
      assets
        ? {
            ...assets,
            segments: assets.segments.map((segment) => {
              const video =
                segment.videos?.find(
                  (video) => video.trackId === selectedTrackId,
                ) ?? (grid ? segment.videos?.[0] : undefined);
              const legacy =
                segment.videos === undefined &&
                selectedTrackId === tracks[0]?.id;
              return {
                ...segment,
                videoUrl: video?.url ?? (legacy ? segment.videoUrl : ''),
                thumbnails:
                  video?.thumbnails ?? (legacy ? segment.thumbnails : []),
              };
            }),
          }
        : null,
    [assets, selectedTrackId, tracks, grid],
  );
  return {
    tracks,
    selectedTrackId,
    selectedDetail,
    selectedAssets,
    grid,
    select,
    toggleGrid: () =>
      setGridChoice((current) => ({
        recordingId: detail.id,
        grid: !(current?.recordingId === detail.id
          ? current.grid
          : tracks.length > 1),
      })),
  };
}
export type StudioVideoSources = ReturnType<typeof useStudioVideoSources>;
