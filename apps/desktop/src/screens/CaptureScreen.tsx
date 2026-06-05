import { useEffect } from 'react';
import { CaptureWorkspace } from '@/features/recorder/components/capture-workspace';
import { useRecorderWorkspaceStore } from '@/features/recorder/recorder-workspace-store';

export function CaptureScreen() {
  const audioSources = useRecorderWorkspaceStore((state) => state.audioSources);
  const captureSources = useRecorderWorkspaceStore(
    (state) => state.captureSources,
  );
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
  const pendingDuplicateCaptureSources = useRecorderWorkspaceStore(
    (state) => state.pendingDuplicateCaptureSources,
  );
  const pendingLifecycleAction = useRecorderWorkspaceStore(
    (state) => state.pendingLifecycleAction,
  );
  const previewFramesBySceneSourceId = useRecorderWorkspaceStore(
    (state) => state.previewFramesBySceneSourceId,
  );
  const previewStatus = useRecorderWorkspaceStore(
    (state) => state.previewStatus,
  );
  const resume = useRecorderWorkspaceStore((state) => state.resume);
  const resolveDuplicateCaptureSource = useRecorderWorkspaceStore(
    (state) => state.resolveDuplicateCaptureSource,
  );
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
  const startPreview = useRecorderWorkspaceStore((state) => state.startPreview);
  const status = useRecorderWorkspaceStore((state) => state.status);
  const stop = useRecorderWorkspaceStore((state) => state.stop);
  const stopPreview = useRecorderWorkspaceStore((state) => state.stopPreview);
  const subscribeToPreviewFrames = useRecorderWorkspaceStore(
    (state) => state.subscribeToPreviewFrames,
  );
  const subscribeToStatusEvents = useRecorderWorkspaceStore(
    (state) => state.subscribeToStatusEvents,
  );
  const updateCaptureSources = useRecorderWorkspaceStore(
    (state) => state.updateCaptureSources,
  );

  useEffect(() => {
    if (loadStatus === 'idle') {
      void load();
    }
  }, [load, loadStatus]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    void subscribeToStatusEvents().then((listener) => {
      if (cancelled) {
        listener();
        return;
      }

      unlisten = listener;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [subscribeToStatusEvents]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    void subscribeToPreviewFrames().then((listener) => {
      if (cancelled) {
        listener();
        return;
      }

      unlisten = listener;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [subscribeToPreviewFrames]);

  useEffect(() => {
    const canPreview =
      loadStatus === 'ready' &&
      !!scene &&
      !isDirty &&
      status?.state === 'idle' &&
      status.finalization === null;

    const previewFailedForScene =
      previewStatus?.sceneId === scene?.id && previewStatus?.state === 'error';

    if (canPreview && !previewFailedForScene) {
      void startPreview(scene.id);
      return;
    }

    if (
      previewStatus?.state === 'starting' ||
      previewStatus?.state === 'live'
    ) {
      void stopPreview();
    }
  }, [
    isDirty,
    loadStatus,
    previewStatus?.sceneId,
    previewStatus?.state,
    scene,
    startPreview,
    status?.finalization,
    status?.state,
    stopPreview,
  ]);

  useEffect(() => {
    return () => {
      void useRecorderWorkspaceStore.getState().stopPreview();
    };
  }, []);

  return (
    <CaptureWorkspace
      audioSources={audioSources}
      captureSources={captureSources}
      desktopAudioSourceId={desktopAudioSourceId}
      error={error}
      isDirty={isDirty}
      loadStatus={loadStatus}
      microphoneAudioSourceId={microphoneAudioSourceId}
      pendingDuplicateCaptureSources={pendingDuplicateCaptureSources}
      pendingLifecycleAction={pendingLifecycleAction}
      previewFramesBySceneSourceId={previewFramesBySceneSourceId}
      previewStatus={previewStatus}
      saveStatus={saveStatus}
      scene={scene}
      selectedSceneSourceId={selectedSceneSourceId}
      status={status}
      onChangeScene={setDraftScene}
      onDeleteSceneSource={deleteSceneSource}
      onPause={() => void pause()}
      onResume={() => void resume()}
      onResolveDuplicateCaptureSource={resolveDuplicateCaptureSource}
      onSave={() => void saveScene()}
      onSelectAudioSource={selectAudioSource}
      onSelectCaptureSources={() => void updateCaptureSources()}
      onSelectSceneSource={selectSceneSource}
      onStart={() => void start()}
      onStop={() => void stop()}
    />
  );
}
