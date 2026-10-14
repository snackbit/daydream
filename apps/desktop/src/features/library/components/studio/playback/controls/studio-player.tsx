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
  SkipForward,
  Volume2,
  VolumeX,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Slider } from '@/components/ui/slider';
import { formatTimestamp } from '../../../../library-format';
import { PlaybackMediaGroup } from '../media/playback-media-group';
import type { RecordingPlaybackController } from '../use-recording-playback-controller';
import { PlaybackPosition } from './playback-position';
import { PlaybackSpeedControl } from './playback-speed-control';
import { SeekTenIcon } from './seek-ten-icon';
import { SegmentScrubber } from './segment-scrubber';

export function StudioPlayer({
  detail,
  onRefreshPlayback,
  playback,
  playbackAssets,
  grid = false,
  sourceLabels,
  sourceLabel,
}: {
  detail: LibraryRecordingDetail;
  onRefreshPlayback: () => void;
  playback: RecordingPlaybackController;
  playbackAssets: LibraryPlaybackAssets | null;
  grid?: boolean;
  sourceLabels?: ReadonlyMap<string, string> | undefined;
  sourceLabel?: string | undefined;
}) {
  const { actions, state } = playback;
  function handleRetryPlayback() {
    actions.seekTimeline(actions.readPlaybackTimeMs(), { resume: true });
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

  if (
    !playbackAssets ||
    (!state.currentPlaybackSegment && !playbackAssets.error)
  ) {
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
  if (currentPlaybackSegment && !currentPlaybackSegment.videoUrl)
    return (
      <div className="bg-muted flex h-full items-center justify-center p-6 text-center text-sm">
        This monitor was not recorded in this segment. Choose another source or
        All monitors.
      </div>
    );
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
  const nextSegment = state.playableSegments[nextSegmentIndex];
  const nextAsset = playbackAssets.segments.find(
    (asset) => asset.segmentId === nextSegment?.id,
  );
  const currentStart =
    (state.currentSegment.mediaStartMs ?? 0) + state.streamStartMs;
  function rewind() {
    actions.seekTimeline(Math.max(0, actions.readPlaybackTimeMs() - 10_000));
  }
  function forward() {
    actions.seekTimeline(
      Math.min(state.totalDurationMs, actions.readPlaybackTimeMs() + 10_000),
    );
  }

  return (
    <div className="bg-background text-foreground flex h-full min-h-0 flex-col">
      <div
        ref={(element) => actions.setFullscreenElement(element)}
        className="relative min-h-0 flex-1 bg-black text-white"
      >
        <div className="h-full min-h-0">
          <PlaybackMediaGroup
            key={`${currentPlaybackSegment.segmentId}-${currentStart}-${state.streamRevision}`}
            active
            asset={currentPlaybackSegment}
            startMs={currentStart}
            revision={state.streamRevision}
            playback={playback}
            label={sourceLabel}
            grid={grid && (currentPlaybackSegment.videos?.length ?? 0) > 1}
            detail={detail}
            labels={sourceLabels}
            timelineOffsetMs={state.currentSegment.playbackOffsetMs}
            streamOffsetMs={state.streamStartMs}
          >
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
                    {state.redactionCountdownSeconds === 1
                      ? 'second'
                      : 'seconds'}
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
          </PlaybackMediaGroup>
          {nextAsset && nextSegment ? (
            <PlaybackMediaGroup
              key={`${nextAsset.segmentId}-${nextSegment.mediaStartMs ?? 0}-${state.streamRevision + 1}`}
              active={false}
              asset={nextAsset}
              startMs={nextSegment.mediaStartMs ?? 0}
              revision={state.streamRevision + 1}
              playback={playback}
              grid={grid && (nextAsset.videos?.length ?? 0) > 1}
              detail={detail}
              labels={sourceLabels}
              timelineOffsetMs={nextSegment.playbackOffsetMs}
              streamOffsetMs={0}
            />
          ) : null}
        </div>
        {state.resumeProgress !== null ? (
          <div className="absolute inset-0 flex items-center justify-center bg-black/70">
            <div className="w-80 max-w-[80%] text-center">
              <div className="text-sm font-medium">
                Not recorded. Next recording at{' '}
                {formatTimestamp(
                  state.playableSegments[nextSegmentIndex]?.startedAtMs ?? null,
                )}
              </div>
              <div role="status" className="mt-2 text-sm">
                Skipping in{' '}
                {Math.max(
                  1,
                  Math.ceil(
                    ((1 - state.resumeProgress) *
                      Math.min(3000, state.currentSegment.pauseAfterMs)) /
                      1000,
                  ),
                )}{' '}
                seconds
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
          isPlaying={state.isPlaying}
          readPlaybackTimeMs={actions.readPlaybackTimeMs}
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
              aria-label="Rewind 10 seconds"
              size="icon"
              variant="ghost"
              onClick={rewind}
            >
              <SeekTenIcon direction="back" />
            </Button>
            <Button
              aria-label={state.playRequested ? 'Pause' : 'Play'}
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
              aria-label="Forward 10 seconds"
              size="icon"
              variant="ghost"
              onClick={forward}
            >
              <SeekTenIcon direction="forward" />
            </Button>
            <span className="ml-2 text-xs tabular-nums">
              <PlaybackPosition playback={playback} />
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
              className="w-16"
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
