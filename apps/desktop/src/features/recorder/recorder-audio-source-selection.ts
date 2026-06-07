import type {
  AudioSource,
  AudioSourceRole,
  RecorderScene,
} from '@daydream/shared';

export function sanitizeSceneAudioSources(
  scene: RecorderScene,
  audioSources: AudioSource[],
): RecorderScene {
  const desktopAudioSourceId = availableAudioSourceId(
    audioSources,
    'desktop-audio',
    scene.desktopAudioSourceId,
  );
  const microphoneAudioSourceId = availableAudioSourceId(
    audioSources,
    'microphone',
    scene.microphoneAudioSourceId,
  );

  if (
    desktopAudioSourceId === scene.desktopAudioSourceId &&
    microphoneAudioSourceId === scene.microphoneAudioSourceId
  ) {
    return scene;
  }

  return {
    ...scene,
    desktopAudioSourceId,
    microphoneAudioSourceId,
  };
}

function availableAudioSourceId(
  audioSources: AudioSource[],
  role: AudioSourceRole,
  sourceId: string | null,
): string | null {
  if (!sourceId) {
    return null;
  }

  return audioSources.some(
    (source) => source.id === sourceId && source.roles.includes(role),
  )
    ? sourceId
    : null;
}
