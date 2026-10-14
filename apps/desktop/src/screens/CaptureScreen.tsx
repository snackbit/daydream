import { useCallback, useEffect } from 'react';
import type { SaveCaptureSetupInput } from '@daydream/shared';
import { SourceCaptureWorkspace } from '@/features/recorder/components/source-capture-workspace';
import { isRecorderError } from '@/features/recorder/recorder-error-utils';
import { useRecorderWorkspaceStore } from '@/features/recorder/recorder-workspace-store';
import { useAsyncErrorBoundary } from '@/lib/use-async-error-boundary';

export function CaptureScreen() {
  const runAsyncAction = useAsyncErrorBoundary();
  const runRecorderAction = useCallback(
    (action: () => Promise<unknown>) => {
      runAsyncAction(async () => {
        try {
          await action();
        } catch (error) {
          // Recorder actions already publish expected native failures to the
          // workspace. Keep session controls available for recovery.
          if (!isRecorderError(error)) throw error;
        }
      });
    },
    [runAsyncAction],
  );
  const audioSources = useRecorderWorkspaceStore((state) => state.audioSources);
  const captureSources = useRecorderWorkspaceStore(
    (state) => state.captureSources,
  );
  const captureSelectionPending = useRecorderWorkspaceStore(
    (state) => state.captureSelectionPending,
  );
  const previewStopInFlight = useRecorderWorkspaceStore(
    (state) => state.previewStopInFlight,
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
  const recordingStats = useRecorderWorkspaceStore(
    (state) => state.recordingStats,
  );
  const recordingStatsError = useRecorderWorkspaceStore(
    (state) => state.recordingStatsError,
  );
  const refreshRecordingStats = useRecorderWorkspaceStore(
    (state) => state.refreshRecordingStats,
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
  const handlePause = useCallback(
    () => runRecorderAction(pause),
    [pause, runRecorderAction],
  );
  const handleResume = useCallback(
    () => runRecorderAction(resume),
    [resume, runRecorderAction],
  );
  const handleSave = useCallback(
    () => runRecorderAction(saveScene),
    [saveScene, runRecorderAction],
  );
  const handleSelectCaptureSources = useCallback(
    () => runRecorderAction(updateCaptureSources),
    [updateCaptureSources, runRecorderAction],
  );
  const handleStart = useCallback(
    (setup?: SaveCaptureSetupInput) => runRecorderAction(() => start(setup)),
    [start, runRecorderAction],
  );
  const handleStop = useCallback(
    () => runRecorderAction(stop),
    [stop, runRecorderAction],
  );

  useEffect(() => {
    if (loadStatus === 'idle') {
      runRecorderAction(load);
    }
  }, [load, loadStatus, runRecorderAction]);

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
      !captureSelectionPending &&
      !previewStopInFlight &&
      !!scene &&
      scene.sources.length > 0 &&
      pendingLifecycleAction === null &&
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
    captureSelectionPending,
    previewStopInFlight,
    loadStatus,
    pendingLifecycleAction,
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

  useEffect(() => {
    if (status?.state !== 'recording') {
      return;
    }

    let cancelled = false;
    let timeoutId: number | null = null;

    async function pollRecordingStats() {
      await refreshRecordingStats();
      if (!cancelled) {
        timeoutId = window.setTimeout(pollRecordingStats, 1_000);
      }
    }

    void pollRecordingStats();

    return () => {
      cancelled = true;
      if (timeoutId !== null) {
        window.clearTimeout(timeoutId);
      }
    };
  }, [refreshRecordingStats, status?.activeRecordingId, status?.state]);

  return (
    <SourceCaptureWorkspace
      audioSources={audioSources}
      captureSources={captureSources}
      captureSelectionPending={captureSelectionPending}
      desktopAudioSourceId={desktopAudioSourceId}
      error={error}
      isDirty={isDirty}
      loadStatus={loadStatus}
      microphoneAudioSourceId={microphoneAudioSourceId}
      pendingDuplicateCaptureSources={pendingDuplicateCaptureSources}
      pendingLifecycleAction={pendingLifecycleAction}
      previewFramesBySceneSourceId={previewFramesBySceneSourceId}
      previewStatus={previewStatus}
      recordingStats={recordingStats}
      recordingStatsError={recordingStatsError}
      saveStatus={saveStatus}
      scene={scene}
      selectedSceneSourceId={selectedSceneSourceId}
      status={status}
      onChangeScene={setDraftScene}
      onDeleteSceneSource={deleteSceneSource}
      onPause={handlePause}
      onResume={handleResume}
      onResolveDuplicateCaptureSource={resolveDuplicateCaptureSource}
      onSave={handleSave}
      onSelectAudioSource={selectAudioSource}
      onSelectCaptureSources={handleSelectCaptureSources}
      onSelectSceneSource={selectSceneSource}
      onStart={handleStart}
      onStop={handleStop}
    />
  );
}
