import { useCallback } from 'react';
import type { LibraryPlaybackAudioAsset } from '@daydream/shared';
import type { RecordingPlaybackController } from '../use-recording-playback-controller';

export interface PlaybackAudioProps {
  audio: LibraryPlaybackAudioAsset;
  src: string;
  actions: Pick<
    RecordingPlaybackController['actions'],
    | 'registerAudioElement'
    | 'handleVideoCanPlay'
    | 'handleBuffering'
    | 'handleMediaError'
  >;
}
export function PlaybackAudio({ audio, src, actions }: PlaybackAudioProps) {
  const { registerAudioElement } = actions;
  const attach = useCallback(
    (element: HTMLAudioElement | null) => {
      registerAudioElement(audio.trackId, element);
    },
    [audio.trackId, registerAudioElement],
  );
  return (
    <audio
      ref={attach}
      src={src}
      preload="metadata"
      onCanPlay={actions.handleVideoCanPlay}
      onSeeked={actions.handleVideoCanPlay}
      onWaiting={actions.handleBuffering}
      onStalled={actions.handleBuffering}
      onError={actions.handleMediaError}
    />
  );
}
