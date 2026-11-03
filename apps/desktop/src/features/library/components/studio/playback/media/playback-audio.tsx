import { useCallback } from 'react';
import type { LibraryPlaybackAudioAsset } from '@daydream/shared';
import type { RecordingPlaybackController } from '../use-recording-playback-controller';

export interface PlaybackAudioProps {
  preload?: 'none' | 'metadata' | 'auto';
  audio: LibraryPlaybackAudioAsset;
  src: string;
  actions: Pick<
    RecordingPlaybackController['actions'],
    | 'registerAudioElement'
    | 'handleVideoCanPlay'
    | 'handleBuffering'
    | 'handleMediaError'
  >;
  active?: boolean;
}
export function PlaybackAudio({
  audio,
  src,
  actions,
  active = true,
  preload = 'auto',
}: PlaybackAudioProps) {
  const { registerAudioElement } = actions;
  const attach = useCallback(
    (element: HTMLAudioElement | null) => {
      if (active) registerAudioElement(audio.trackId, element);
    },
    [audio.trackId, registerAudioElement, active],
  );
  return (
    <audio
      ref={attach}
      src={src}
      preload={preload}
      muted={!active}
      onCanPlay={active ? actions.handleVideoCanPlay : undefined}
      onSeeked={active ? actions.handleVideoCanPlay : undefined}
      onWaiting={active ? actions.handleBuffering : undefined}
      onStalled={active ? actions.handleBuffering : undefined}
      onError={active ? actions.handleMediaError : undefined}
    />
  );
}
