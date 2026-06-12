import type {
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
} from '@daydream/shared';
import {
  AlertTriangle,
  Lock,
  Maximize,
  Pause,
  Play,
  SkipBack,
  SkipForward,
  Volume2,
  VolumeX,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { formatDuration, formatTimestamp } from '../../library-format';
import { SegmentScrubber } from './segment-scrubber';
import type { RecordingPlaybackController } from './use-recording-playback-controller';

export function StudioPlayer({
  detail,
  playback,
  playbackAssets,
}: {
  detail: LibraryRecordingDetail;
  playback: RecordingPlaybackController;
  playbackAssets: LibraryPlaybackAssets | null;
}) {
  const { actions, state } = playback;

  if (detail.availability !== 'available' || !state.currentSegment) {
    const locked = detail.availability === 'locked';
    return (
      <div className="bg-muted text-muted-foreground flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
        {locked ? (
          <Lock className="size-8" />
        ) : (
          <AlertTriangle className="size-8" />
        )}
        <div>
          <div className="text-foreground text-sm font-medium">
            {locked ? 'Recording in progress' : 'Missing video files'}
          </div>
          <p className="mt-1 max-w-md text-xs">
            {locked
              ? 'This recording is still controlled by the recorder and cannot be played or deleted from Library.'
              : 'This recording exists in the database, but Daydream cannot find the local MKV files needed for playback.'}
          </p>
        </div>
      </div>
    );
  }

  if (!playbackAssets) {
    return (
      <div className="bg-muted text-muted-foreground flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
        <div className="text-foreground text-sm font-medium">
          Preparing playback...
        </div>
        <p className="max-w-md text-xs">
          Daydream is preparing browser-playable media from the local MKV
          segment.
        </p>
      </div>
    );
  }

  const currentPlaybackSegment = state.currentPlaybackSegment;
  if (playbackAssets.error || !currentPlaybackSegment) {
    return (
      <div className="bg-muted text-muted-foreground flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
        <AlertTriangle className="size-8" />
        <div>
          <div className="text-foreground text-sm font-medium">
            Playback preparation failed
          </div>
          <p className="mt-1 max-w-md text-xs">
            {playbackAssets.error ??
              'Daydream could not create playable media for this segment.'}
          </p>
        </div>
      </div>
    );
  }

  const nextSegmentIndex =
    state.playableSegments.indexOf(state.currentSegment) + 1;

  return (
    <div className="flex h-full flex-col bg-black text-white">
      <div
        ref={(element) => actions.setFullscreenElement(element)}
        className="relative min-h-0 flex-1 bg-black"
      >
        <video
          key={`${currentPlaybackSegment.segmentId}-${state.streamStartMs}-${state.streamRevision}-video`}
          ref={(element) => actions.setVideoElement(element)}
          className="h-full w-full object-contain"
          preload="metadata"
          src={streamUrl(
            currentPlaybackSegment.videoUrl,
            state.streamStartMs,
            state.streamRevision,
          )}
          onPause={() => actions.handleVideoPause()}
          onPlay={() => actions.handleVideoPlay()}
          onTimeUpdate={(event) =>
            actions.handleVideoTimeUpdate(event.currentTarget)
          }
          onCanPlay={() => actions.handleVideoCanPlay()}
          onEnded={() => actions.handleEnded()}
          onError={() => actions.handleVideoError()}
        />
        {currentPlaybackSegment.audio.map((audio) => (
          <audio
            key={`${currentPlaybackSegment.segmentId}-${audio.trackId}-${state.streamStartMs}-${state.streamRevision}`}
            ref={(element) =>
              actions.registerAudioElement(audio.trackId, element)
            }
            preload="metadata"
            src={streamUrl(
              audio.url,
              state.streamStartMs,
              state.streamRevision,
            )}
          />
        ))}
        {state.redactedVideoActive ? (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-neutral-700 text-sm font-medium text-white">
            <div>Redacted</div>
            {state.nextUnredactedVideoTimelineMs !== null ? (
              <Button
                className="gap-2"
                size="sm"
                variant="secondary"
                onClick={() => actions.skipToNextUnredactedVideo()}
              >
                <SkipForward className="size-3.5" />
                Skip redacted part
              </Button>
            ) : null}
          </div>
        ) : null}
        {state.resumeProgress !== null ? (
          <div className="absolute inset-0 flex items-center justify-center bg-black/70">
            <div className="w-80 max-w-[80%] text-center">
              <div className="text-sm font-medium">
                Recording paused. Resuming at{' '}
                {formatTimestamp(
                  state.playableSegments[nextSegmentIndex]?.startedAtMs ?? null,
                )}
              </div>
              <div className="mt-3 h-1 overflow-hidden rounded-full bg-white/20">
                <div
                  className="h-full bg-white"
                  style={{ width: `${state.resumeProgress * 100}%` }}
                />
              </div>
              <Button
                className="mt-4 gap-2"
                size="sm"
                variant="secondary"
                onClick={() => actions.playNowAfterPause()}
              >
                <SkipForward className="size-3.5" />
                Play now
              </Button>
            </div>
          </div>
        ) : null}
        {state.playbackError ? (
          <div className="absolute inset-0 flex items-center justify-center bg-black/80 p-6 text-center">
            <div>
              <div className="text-sm font-medium">Playback failed</div>
              <p className="mt-1 max-w-md text-xs text-white/70">
                {state.playbackError}
              </p>
            </div>
          </div>
        ) : null}
      </div>

      <div className="flex shrink-0 flex-col gap-2 border-t border-white/10 bg-black/90 p-3">
        <SegmentScrubber
          currentTimelineMs={Math.min(
            state.displayedTimelineMs,
            state.totalDurationMs,
          )}
          playbackAssets={playbackAssets}
          segments={state.playableSegments}
          totalDurationMs={Math.max(state.totalDurationMs, 1)}
          onCommit={(timelineMs) => actions.commitTimelineInput(timelineMs)}
          onScrubUpdate={(timelineMs) =>
            actions.handleTimelineInput(timelineMs)
          }
        />
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-1">
            <Button
              aria-label="Skip back"
              size="icon"
              variant="ghost"
              onClick={() =>
                actions.seekTimeline(
                  Math.max(0, state.displayedTimelineMs - 10_000),
                )
              }
            >
              <SkipBack className="size-4" />
            </Button>
            <Button
              aria-label={state.isPlaying ? 'Pause' : 'Play'}
              size="icon"
              variant="secondary"
              onClick={() => void actions.togglePlayback()}
            >
              {state.isPlaying ? (
                <Pause className="size-4" />
              ) : (
                <Play className="size-4" />
              )}
            </Button>
            <Button
              aria-label="Skip forward"
              size="icon"
              variant="ghost"
              onClick={() =>
                actions.seekTimeline(
                  Math.min(
                    state.totalDurationMs,
                    state.displayedTimelineMs + 10_000,
                  ),
                )
              }
            >
              <SkipForward className="size-4" />
            </Button>
            <span className="ml-2 text-xs tabular-nums">
              {formatDuration(state.displayedTimelineMs)} /{' '}
              {formatDuration(state.totalDurationMs)}
            </span>
          </div>

          <div className="flex items-center gap-2">
            <Button
              aria-label={state.isMuted ? 'Unmute' : 'Mute'}
              size="icon"
              variant="ghost"
              onClick={() => actions.setIsMuted((muted) => !muted)}
            >
              {state.isMuted ? (
                <VolumeX className="size-4" />
              ) : (
                <Volume2 className="size-4" />
              )}
            </Button>
            <input
              aria-label="Volume"
              className="w-24 accent-white"
              max={1}
              min={0}
              step={0.05}
              type="range"
              value={state.volume}
              onChange={(event) =>
                actions.setVolume(Number(event.currentTarget.value))
              }
            />
            <Button
              aria-label="Fullscreen"
              size="icon"
              variant="ghost"
              onClick={() => void actions.requestFullscreen()}
            >
              <Maximize className="size-4" />
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

function streamUrl(url: string, startMs: number, revision: number): string {
  const separator = url.includes('?') ? '&' : '?';

  return `${url}${separator}startMs=${Math.max(
    0,
    Math.floor(startMs),
  )}&request=${Math.max(0, Math.floor(revision))}`;
}
