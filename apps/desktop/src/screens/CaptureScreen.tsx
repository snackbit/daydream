import { useEffect } from 'react';
import { CaptureWorkspace } from '@/features/recorder/components/capture-workspace';
import { useRecorderWorkspaceStore } from '@/features/recorder/recorder-workspace-store';

export function CaptureScreen() {
  const audioSources = useRecorderWorkspaceStore((state) => state.audioSources);
  const desktopAudioSourceId = useRecorderWorkspaceStore(
    (state) => state.desktopAudioSourceId,
  );
  const error = useRecorderWorkspaceStore((state) => state.error);
  const isDirty = useRecorderWorkspaceStore((state) => state.isDirty);
  const deleteSceneSource = useRecorderWorkspaceStore(
    (state) => state.deleteSceneSource,
  );
  const load = useRecorderWorkspaceStore((state) => state.load);
  const loadStatus = useRecorderWorkspaceStore((state) => state.loadStatus);
  const microphoneAudioSourceId = useRecorderWorkspaceStore(
    (state) => state.microphoneAudioSourceId,
  );
  const pause = useRecorderWorkspaceStore((state) => state.pause);
  const resume = useRecorderWorkspaceStore((state) => state.resume);
  const saveScene = useRecorderWorkspaceStore((state) => state.saveScene);
  const saveStatus = useRecorderWorkspaceStore((state) => state.saveStatus);
  const scene = useRecorderWorkspaceStore((state) => state.scene);
  const selectAudioSource = useRecorderWorkspaceStore(
    (state) => state.selectAudioSource,
  );
  const selectSceneSource = useRecorderWorkspaceStore(
    (state) => state.selectSceneSource,
  );
  const selectedSceneSourceId = useRecorderWorkspaceStore(
    (state) => state.selectedSceneSourceId,
  );
  const setDraftScene = useRecorderWorkspaceStore(
    (state) => state.setDraftScene,
  );
  const start = useRecorderWorkspaceStore((state) => state.start);
  const status = useRecorderWorkspaceStore((state) => state.status);
  const stop = useRecorderWorkspaceStore((state) => state.stop);
  const updateCaptureSources = useRecorderWorkspaceStore(
    (state) => state.updateCaptureSources,
  );

  useEffect(() => {
    if (loadStatus === 'idle') {
      void load();
    }
  }, [load, loadStatus]);

  return (
    <CaptureWorkspace
      audioSources={audioSources}
      desktopAudioSourceId={desktopAudioSourceId}
      error={error}
      isDirty={isDirty}
      loadStatus={loadStatus}
      microphoneAudioSourceId={microphoneAudioSourceId}
      saveStatus={saveStatus}
      scene={scene}
      selectedSceneSourceId={selectedSceneSourceId}
      status={status}
      onChangeScene={setDraftScene}
      onDeleteSceneSource={deleteSceneSource}
      onPause={() => void pause()}
      onResume={() => void resume()}
      onSave={() => void saveScene()}
      onSelectAudioSource={selectAudioSource}
      onSelectCaptureSources={() => void updateCaptureSources()}
      onSelectSceneSource={selectSceneSource}
      onStart={() => void start()}
      onStop={() => void stop()}
    />
  );
}
