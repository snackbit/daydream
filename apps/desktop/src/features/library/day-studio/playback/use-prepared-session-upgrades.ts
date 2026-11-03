import { useEffect, useEffectEvent } from 'react';
import type { RecordingPlaybackController } from '../../components/studio/playback/use-recording-playback-controller';
import type { useDayPlaybackSessions } from '../use-day-playback-sessions';

/** Upgrade paused media without interrupting playback or replacing a newer seek. */
export function usePreparedSessionUpgrades(
  activeId: string | null,
  playback: RecordingPlaybackController,
  sessions: ReturnType<typeof useDayPlaybackSessions>,
) {
  const canApply = useEffectEvent(
    (id: string, revision: number) =>
      activeId === id &&
      !playback.state.isPlaying &&
      !playback.state.playRequested &&
      playback.state.seekRevision === revision,
  );
  const restore = useEffectEvent(
    (id: string, revision: number, time: number) => {
      if (canApply(id, revision))
        playback.actions.seekTimeline(time, { resume: false });
    },
  );
  const { ensure, preparedIds } = sessions;
  const { isPlaying, playRequested, seekRevision } = playback.state;
  const { readPlaybackTimeMs } = playback.actions;
  useEffect(() => {
    if (!activeId || isPlaying || playRequested || !preparedIds.has(activeId))
      return;
    const time = readPlaybackTimeMs();
    void ensure(activeId, true, () => canApply(activeId, seekRevision))
      .then(() => restore(activeId, seekRevision, time))
      .catch(() => undefined);
  }, [
    activeId,
    isPlaying,
    playRequested,
    preparedIds,
    ensure,
    readPlaybackTimeMs,
    seekRevision,
  ]);
}
