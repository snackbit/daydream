import { formatDuration } from '../../../../library-format';
import { useLivePlaybackTime } from '../clock/use-live-playback-time';
import type { RecordingPlaybackController } from '../use-recording-playback-controller';

export interface PlaybackPositionProps {
  playback: RecordingPlaybackController;
}
export function PlaybackPosition({ playback }: PlaybackPositionProps) {
  const time = useLivePlaybackTime(
    playback.state.displayedTimelineMs,
    playback.state.isPlaying,
    playback.actions.readPlaybackTimeMs,
  );
  return (
    <>
      {formatDuration(time)} / {formatDuration(playback.state.totalDurationMs)}
    </>
  );
}
