import { useMemo } from 'react';
import type {
  AudioSource,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import { CaptureResizableLayout } from './capture-resizable-layout';
import { CaptureToolbar } from './capture-toolbar';
import { RecorderErrorAlert } from './recorder-error-alert';

export interface CaptureWorkspaceProps {
  audioSources: AudioSource[];
  desktopAudioSourceId: string | null;
  error: string | null;
  isDirty: boolean;
  loadStatus: string;
  microphoneAudioSourceId: string | null;
  onChangeScene: (scene: RecorderScene) => void;
  onDeleteSceneSource: (sceneSourceId: string) => void;
  onPause: () => void;
  onResume: () => void;
  onSave: () => void;
  onSelectAudioSource: (
    role: 'desktop-audio' | 'microphone',
    sourceId: string | null,
  ) => void;
  onSelectCaptureSources: () => void;
  onSelectSceneSource: (sourceId: string | null) => void;
  onStart: () => void;
  onStop: () => void;
  saveStatus: string;
  scene: RecorderScene | null;
  selectedSceneSourceId: string | null;
  status: RecorderStatus | null;
}

export function CaptureWorkspace({
  audioSources,
  desktopAudioSourceId,
  error,
  isDirty,
  loadStatus,
  microphoneAudioSourceId,
  onChangeScene,
  onDeleteSceneSource,
  onPause,
  onResume,
  onSave,
  onSelectAudioSource,
  onSelectCaptureSources,
  onSelectSceneSource,
  onStart,
  onStop,
  saveStatus,
  scene,
  selectedSceneSourceId,
  status,
}: CaptureWorkspaceProps) {
  const selectedSceneSource = useMemo(
    () =>
      scene?.sources.find((source) => source.id === selectedSceneSourceId) ??
      null,
    [scene, selectedSceneSourceId],
  );

  return (
    <section className="flex min-h-full flex-col">
      <CaptureToolbar
        isDirty={isDirty}
        loadStatus={loadStatus}
        saveStatus={saveStatus}
        scene={scene}
        status={status}
        onPause={onPause}
        onResume={onResume}
        onSave={onSave}
        onStart={onStart}
        onStop={onStop}
      />

      {error ? <RecorderErrorAlert message={error} /> : null}

      <CaptureResizableLayout
        audioSources={audioSources}
        desktopAudioSourceId={desktopAudioSourceId}
        loadStatus={loadStatus}
        microphoneAudioSourceId={microphoneAudioSourceId}
        scene={scene}
        selectedSceneSource={selectedSceneSource}
        selectedSceneSourceId={selectedSceneSourceId}
        onChangeScene={onChangeScene}
        onDeleteSceneSource={onDeleteSceneSource}
        onSelectAudioSource={onSelectAudioSource}
        onSelectCaptureSources={onSelectCaptureSources}
        onSelectSceneSource={onSelectSceneSource}
      />
    </section>
  );
}
