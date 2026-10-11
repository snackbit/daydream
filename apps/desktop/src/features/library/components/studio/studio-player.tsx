import { useCallback } from 'react';
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
  RefreshCw,
  SkipBack,
  SkipForward,
  Volume2,
  VolumeX,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Slider } from '@/components/ui/slider';
import { formatDuration, formatTimestamp } from '../../library-format';
import { PlaybackAudio } from './playback/playback-audio';
import { PlaybackSpeedControl } from './playback/playback-speed-control';
import { SegmentScrubber } from './segment-scrubber';
import type { RecordingPlaybackController } from './use-recording-playback-controller';

export function StudioPlayer({
  detail,
  onRefreshPlayback,
  playback,
  playbackAssets,
}: {
  detail: LibraryRecordingDetail;
  onRefreshPlayback: () => void;
  playback: RecordingPlaybackController;
  playbackAssets: LibraryPlaybackAssets | null;
}) {
  const { actions, state } = playback;
  const { setVideoElement } = actions;
  const attachVideo = useCallback(
    (element: HTMLVideoElement | null) => {
      setVideoElement(element);
    },
    [setVideoElement],
  );
  function handleRetryPlayback() {
    actions.seekTimeline(state.displayedTimelineMs, { resume: true });
  }

  function handleVolumeChange(values: number[]) {
    const [volume] = values;
    if (volume !== undefined) {
      actions.setVolume(volume);
    }
  }

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
        <Button
          className="gap-2"
          size="sm"
          variant="outline"
          onClick={onRefreshPlayback}
        >
          <RefreshCw className="size-3.5" />
          Retry loading
        </Button>
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
        <Button
          className="gap-2"
          size="sm"
          variant="outline"
          onClick={onRefreshPlayback}
        >
          <RefreshCw className="size-3.5" />
          Retry loading
        </Button>
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
        <Button
          className="gap-2"
          size="sm"
          variant="outline"
          onClick={onRefreshPlayback}
        >
          <RefreshCw className="size-3.5" />
          Retry loading
        </Button>
      </div>
    );
  }

  const nextSegmentIndex =
    state.playableSegments.indexOf(state.currentSegment) + 1;

  return (
    <div className="bg-background text-foreground flex h-full min-h-0 flex-col">
      <div
        ref={(element) => actions.setFullscreenElement(element)}
        className="relative min-h-0 flex-1 bg-black text-white"
      >
        <video
          key={`${currentPlaybackSegment.segmentId}-${state.streamStartMs}-${state.streamRevision}-video`}
          ref={attachVideo}
          className="h-full w-full object-contain"
          preload="metadata"
          src={streamUrl(
            currentPlaybackSegment.videoUrl,
            state.streamStartMs,
            state.streamRevision,
          )}
          onPause={actions.handleVideoPause}
          onPlay={actions.handleVideoPlay}
          onTimeUpdate={(event) =>
            actions.handleVideoTimeUpdate(event.currentTarget)
          }
          onCanPlay={actions.handleVideoCanPlay}
          onSeeked={actions.handleVideoCanPlay}
          onWaiting={actions.handleBuffering}
          onStalled={actions.handleBuffering}
          onEnded={actions.handleEnded}
          onError={actions.handleMediaError}
        />
        {currentPlaybackSegment.audio.map((audio) => (
          <PlaybackAudio
            key={`${currentPlaybackSegment.segmentId}-${audio.trackId}-${state.streamStartMs}-${state.streamRevision}`}
            audio={audio}
            actions={actions}
            src={streamUrl(
              audio.url,
              state.streamStartMs,
              state.streamRevision,
            )}
          />
        ))}
        {state.redactedVideoActive ? (
          <div
            aria-label="Redacted video"
            className="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-neutral-700 text-sm font-medium text-white"
            role="group"
          >
            <div>Redacted</div>
            {state.redactionCountdownSeconds !== null ? (
              <div role="status">
                Skipping in {state.redactionCountdownSeconds}{' '}
                {state.redactionCountdownSeconds === 1 ? 'second' : 'seconds'}
              </div>
            ) : null}
            {state.redactionSkipTargetMs !== null ? (
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
              <Button
                className="mt-3 gap-2"
                size="sm"
                variant="secondary"
                onClick={handleRetryPlayback}
              >
                <RefreshCw className="size-3.5" />
                Retry playback
              </Button>
            </div>
          </div>
        ) : null}
      </div>

      <div className="bg-background text-foreground flex shrink-0 flex-col gap-4 border-t p-5 sm:p-6">
        <SegmentScrubber
          privacy={detail}
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
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div className="flex flex-wrap items-center gap-2">
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
              aria-label={state.playRequested ? 'Pause' : 'Play'}
              disabled={state.resumeProgress !== null}
              size="icon"
              variant="secondary"
              onClick={() => void actions.togglePlayback()}
            >
              {state.playRequested ? (
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

          <div className="flex flex-wrap items-center gap-3">
            <PlaybackSpeedControl
              value={state.playbackRate}
              onChange={actions.setPlaybackRate}
            />
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
            <Slider
              aria-label="Volume"
              className="w-24"
              max={1}
              min={0}
              step={0.05}
              value={[state.volume]}
              onValueChange={handleVolumeChange}
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
