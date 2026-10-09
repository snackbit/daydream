import type {
  LibraryRecordingSegment,
  LibraryRecordingTrack,
  LibraryWaveformTileTrack,
} from '@daydream/shared';
import { VideoTimelineThumbnail } from './video-timeline-thumbnail';

export interface TrackClipMediaProps {
  segment: LibraryRecordingSegment;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
  thumbnails: Array<{ offsetMs: number; url: string }>;
  track: LibraryRecordingTrack;
  waveformTrack: LibraryWaveformTileTrack | undefined;
}

export function TrackClipMedia({
  segment,
  thumbnailStatus,
  thumbnails,
  track,
  waveformTrack,
}: TrackClipMediaProps) {
  if (track.kind === 'video') {
    return (
      <VideoThumbnailStrip
        isLoading={thumbnailStatus === 'loading'}
        thumbnails={thumbnails}
      />
    );
  }

  if (waveformTrack) {
    return (
      <WaveformTileLayer segment={segment} waveformTrack={waveformTrack} />
    );
  }

  return <WaveformSilenceLine />;
}

export function WaveformSilenceLine() {
  return (
    <div
      aria-label="Audio silence baseline"
      className="bg-primary/30 absolute inset-x-0 top-1/2 h-px -translate-y-1/2"
    />
  );
}

function WaveformTileLayer({
  segment,
  waveformTrack,
}: {
  segment: LibraryRecordingSegment;
  waveformTrack: LibraryWaveformTileTrack;
}) {
  const segmentDurationMs = Math.max(1, segment.durationMs ?? 0);
  const tiles = waveformTrack.tiles.filter(
    (tile) =>
      tile.segmentId === segment.id ||
      segment.fragmentIds?.includes(tile.segmentId),
  );
  if (tiles.length === 0) {
    return <WaveformSilenceLine />;
  }

  return (
    <div className="absolute inset-0">
      <WaveformSilenceLine />
      {tiles.map((tile) => {
        const tileStartInSegmentMs =
          tile.timelineStartMs - segment.playbackOffsetMs;
        const clippedStartMs = Math.max(0, tileStartInSegmentMs);
        const clippedEndMs = Math.min(
          segmentDurationMs,
          tileStartInSegmentMs + tile.durationMs,
        );
        if (clippedEndMs <= clippedStartMs) {
          return null;
        }

        return (
          <div
            className="absolute inset-y-0"
            key={`${tile.segmentId}-${tile.streamIndex}-${tile.binMs}-${tile.timelineStartMs}`}
            style={{
              left: `${(clippedStartMs / segmentDurationMs) * 100}%`,
              width: `${
                ((clippedEndMs - clippedStartMs) / segmentDurationMs) * 100
              }%`,
            }}
          >
            <WaveformShape samples={tile.peakSamples} />
          </div>
        );
      })}
    </div>
  );
}

function VideoThumbnailStrip({
  isLoading,
  thumbnails,
}: {
  isLoading: boolean;
  thumbnails: Array<{ offsetMs: number; url: string }>;
}) {
  if (thumbnails.length === 0) {
    return (
      <div className="text-muted-foreground flex h-full items-center justify-center px-2 text-xs">
        {isLoading ? 'Thumbnails loading...' : 'Thumbnails unavailable'}
      </div>
    );
  }

  return (
    <div className="flex h-full w-full">
      {thumbnails.map((thumbnail) => (
        <VideoTimelineThumbnail
          key={`${thumbnail.url}-${thumbnail.offsetMs}`}
          url={thumbnail.url}
        />
      ))}
    </div>
  );
}

function WaveformShape({ samples }: { samples: number[] }) {
  if (samples.length === 0) {
    return null;
  }
  const points = samples.map((sample, index) => {
    const x = samples.length === 1 ? 0 : (index / (samples.length - 1)) * 100;
    const y = 50 - Math.max(0, Math.min(1, sample)) * 48;

    return samples.length === 1
      ? `0,${y.toFixed(3)} 100,${y.toFixed(3)}`
      : `${x.toFixed(3)},${y.toFixed(3)}`;
  });
  const reflected = [...samples].reverse().map((sample, index) => {
    const x =
      samples.length === 1
        ? 0
        : ((samples.length - 1 - index) / (samples.length - 1)) * 100;
    const y = 50 + Math.max(0, Math.min(1, sample)) * 48;

    return samples.length === 1
      ? `100,${y.toFixed(3)} 0,${y.toFixed(3)}`
      : `${x.toFixed(3)},${y.toFixed(3)}`;
  });

  return (
    <svg
      aria-label="Audio waveform"
      className="size-full"
      preserveAspectRatio="none"
      viewBox="0 0 100 100"
    >
      <polygon
        fill="currentColor"
        opacity="0.72"
        points={`${points.join(' ')} ${reflected.join(' ')}`}
      />
    </svg>
  );
}
