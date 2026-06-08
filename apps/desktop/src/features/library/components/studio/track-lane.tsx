import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
  LibraryRecordingSegment,
  LibraryRecordingTrack,
  LibraryWaveformTileTrack,
  LibraryWaveformTiles,
} from '@daydream/shared';
import { Volume2, VolumeX } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { TRACK_LANE_HEIGHT_PX } from './track-timeline-math';

export function TrackLane({
  detail,
  mutedTrackIds,
  onToggleTrackMuted,
  playbackAssets,
  thumbnailStatus,
  timelineWidthPx,
  totalDurationMs,
  track,
  waveformTiles,
}: {
  detail: LibraryRecordingDetail;
  mutedTrackIds: Set<string>;
  onToggleTrackMuted: (trackId: string) => void;
  playbackAssets: LibraryPlaybackAssets | null;
  thumbnailStatus: 'error' | 'idle' | 'loading' | 'ready';
  timelineWidthPx: number;
  totalDurationMs: number;
  track: LibraryRecordingTrack;
  waveformTiles: LibraryWaveformTiles | null;
}) {
  const waveformTrack = waveformTiles?.tracks.find(
    (waveform) =>
      waveform.trackId === track.kind || waveform.kind === track.kind,
  );
  const audioTrack = track.kind !== 'video';
  const muted = mutedTrackIds.has(track.id);

  return (
    <>
      <div
        className="bg-background sticky left-0 z-10 flex min-w-0 items-start gap-2 border-r border-b p-2"
        style={{ height: TRACK_LANE_HEIGHT_PX }}
      >
        {audioTrack ? (
          <Button
            aria-label={muted ? `Unmute ${track.label}` : `Mute ${track.label}`}
            data-library-action
            size="icon"
            variant={muted ? 'outline' : 'ghost'}
            onClick={() => onToggleTrackMuted(track.id)}
          >
            {muted ? (
              <VolumeX className="size-3.5" />
            ) : (
              <Volume2 className="size-3.5" />
            )}
          </Button>
        ) : null}
        <div className="min-w-0">
          <div className="truncate text-xs font-medium">{track.label}</div>
          <div className="text-muted-foreground truncate text-xs">
            {track.codec ?? track.description}
          </div>
        </div>
      </div>
      <div
        className="bg-muted relative overflow-hidden border-b"
        style={{ height: TRACK_LANE_HEIGHT_PX, width: timelineWidthPx }}
      >
        {detail.segments.map((segment) => (
          <div
            className={cn(
              'absolute top-1 bottom-1 overflow-hidden border',
              segment.fileExists &&
                segment.state === 'finalized' &&
                track.kind !== 'video'
                ? 'bg-primary/10 border-primary/30'
                : 'bg-muted-foreground/10 border-muted-foreground/20',
              segment.fileExists &&
                segment.state === 'finalized' &&
                track.kind === 'video' &&
                'bg-background',
            )}
            key={`${track.id}-${segment.id}`}
            style={{
              left:
                (segment.playbackOffsetMs / totalDurationMs) * timelineWidthPx,
              width:
                ((segment.durationMs ?? 0) / totalDurationMs) * timelineWidthPx,
            }}
            title={`Segment ${segment.segmentIndex}`}
          >
            {track.kind === 'video' ? (
              <VideoThumbnailStrip
                isLoading={thumbnailStatus === 'loading'}
                thumbnails={
                  playbackAssets?.segments.find(
                    (playbackSegment) =>
                      playbackSegment.segmentId === segment.id,
                  )?.thumbnails ?? []
                }
              />
            ) : waveformTrack ? (
              <WaveformTileLayer
                segment={segment}
                waveformTrack={waveformTrack}
              />
            ) : audioTrack ? (
              <WaveformSilenceLine />
            ) : null}
          </div>
        ))}
      </div>
    </>
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
    (tile) => tile.segmentId === segment.id,
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

function WaveformSilenceLine() {
  return (
    <div
      aria-label="Audio silence baseline"
      className="bg-primary/30 absolute inset-x-0 top-1/2 h-px -translate-y-1/2"
    />
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
      <div className="text-muted-foreground flex h-full items-center justify-center px-2 text-[0.625rem]">
        {isLoading ? 'Thumbnails loading...' : 'Thumbnails unavailable'}
      </div>
    );
  }

  return (
    <div className="flex h-full w-full">
      {thumbnails.map((thumbnail) => (
        <img
          alt=""
          aria-label="Video timeline thumbnail"
          className="h-full min-w-0 flex-1 border-r object-cover last:border-r-0"
          key={`${thumbnail.url}-${thumbnail.offsetMs}`}
          src={thumbnail.url}
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

    return `${x.toFixed(3)},${y.toFixed(3)}`;
  });
  const reflected = [...samples].reverse().map((sample, index) => {
    const x =
      samples.length === 1
        ? 0
        : ((samples.length - 1 - index) / (samples.length - 1)) * 100;
    const y = 50 + Math.max(0, Math.min(1, sample)) * 48;

    return `${x.toFixed(3)},${y.toFixed(3)}`;
  });

  return (
    <svg
      aria-label="Audio waveform"
      className="text-primary/75 h-full w-full"
      preserveAspectRatio="none"
      viewBox="0 0 100 100"
    >
      <polygon
        className="fill-current"
        points={`${points.join(' ')} ${reflected.join(' ')}`}
      />
    </svg>
  );
}
