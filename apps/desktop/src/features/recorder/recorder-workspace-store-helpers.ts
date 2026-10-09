import type {
  CaptureSource,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import { runAsyncStoreAction, sleep } from '@/lib/async-store-action';
import { sanitizeSceneAudioSources } from './recorder-audio-source-selection';
import {
  getAudioSources,
  getCaptureSources,
  getRecorderPreviewStatus,
  getRecorderScene,
  getRecorderStatus,
} from './recorder-client';
import { errorMessage } from './recorder-error-utils';
import {
  isRecoverableRecorderActionError,
  recorderConfigurationLocked,
  statusBlocksLifecycle,
  statusShowsCommandInProgress,
} from './recorder-lifecycle-state';
import { recorderStatusIsCurrent } from './recorder-status-order';
import type {
  RecorderLifecycleAction,
  RecorderWorkspaceGet,
  RecorderWorkspaceSet,
  RecorderWorkspaceState,
} from './recorder-workspace-store-types';
import {
  classifyCaptureSourceMerge,
  createDefaultRecorderScene,
  mergeCaptureSourcesIntoScene,
} from './scene-layout';

const FINALIZATION_POLL_INTERVAL_MS = 250;
const FINALIZATION_POLL_TIMEOUT_MS = 25_000;

export function workspaceConfigurationLocked(
  state: RecorderWorkspaceState,
): boolean {
  return recorderConfigurationLocked(
    state.pendingLifecycleAction,
    state.status,
  );
}

export function selectExistingSceneSource(
  scene: RecorderScene,
  sceneSourceId: string | null,
): string | null {
  return scene.sources.some((source) => source.id === sceneSourceId)
    ? sceneSourceId
    : (scene.sources[0]?.id ?? null);
}

export async function loadRecorderWorkspaceSnapshot(
  savedSceneOverride?: RecorderScene | null,
) {
  const [
    status,
    previewStatus,
    captureSourceList,
    audioSourceList,
    savedScene,
  ] = await Promise.all([
    getRecorderStatus(),
    getRecorderPreviewStatus(),
    getCaptureSources(),
    getAudioSources(),
    savedSceneOverride === undefined
      ? getRecorderScene()
      : Promise.resolve(savedSceneOverride),
  ]);

  return {
    audioSourceList,
    captureSourceList,
    previewStatus,
    savedScene,
    status,
  };
}

export function applyLoadedRecorderWorkspace(
  set: RecorderWorkspaceSet,
  snapshot: Awaited<ReturnType<typeof loadRecorderWorkspaceSnapshot>>,
): void {
  const {
    audioSourceList,
    captureSourceList,
    previewStatus,
    savedScene,
    status,
  } = snapshot;
  const restoredScene =
    savedScene ?? createDefaultRecorderScene(captureSourceList.sources);
  const scene = sanitizeSceneAudioSources(
    restoredScene,
    audioSourceList.sources,
  );
  const droppedSavedAudioSource =
    !!savedScene &&
    (scene.desktopAudioSourceId !== savedScene.desktopAudioSourceId ||
      scene.microphoneAudioSourceId !== savedScene.microphoneAudioSourceId);
  set((current) => ({
    audioSources: audioSourceList.sources,
    captureSources: captureSourceList.sources,
    desktopAudioSourceId: scene.desktopAudioSourceId,
    error: recorderStatusIsCurrent(current.status, status)
      ? null
      : current.error,
    isDirty:
      droppedSavedAudioSource ||
      (!savedScene && captureSourceList.sources.length > 0),
    loadStatus: 'ready',
    microphoneAudioSourceId: scene.microphoneAudioSourceId,
    previewFramesBySceneSourceId: {},
    previewStatus,
    recordingStats: null,
    recordingStatsError: null,
    scene,
    selectedSceneSourceId: scene.sources[0]?.id ?? null,
    status: recorderStatusIsCurrent(current.status, status)
      ? status
      : current.status,
  }));
}

export function applySelectedCaptureSources(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
  sources: CaptureSource[],
): void {
  const currentScene = get().scene ?? createDefaultRecorderScene([]);
  const classification = classifyCaptureSourceMerge(currentScene, sources);
  const merged = mergeCaptureSourcesIntoScene(
    currentScene,
    classification.newSources,
  );
  const existingIds = new Set(currentScene.sources.map((source) => source.id));
  const scene = currentScene.independentVideo
    ? {
        ...merged,
        sources: merged.sources.map((source) =>
          !existingIds.has(source.id) &&
          source.sourceWidth &&
          source.sourceHeight
            ? {
                ...source,
                rect: {
                  x: 0,
                  y: 0,
                  width:
                    Math.floor(
                      (source.sourceWidth -
                        source.crop.left -
                        source.crop.right) /
                        2,
                    ) * 2,
                  height:
                    Math.floor(
                      (source.sourceHeight -
                        source.crop.top -
                        source.crop.bottom) /
                        2,
                    ) * 2,
                },
              }
            : source,
        ),
      }
    : merged;
  set({
    captureSources: sources,
    error: null,
    isDirty:
      scene !== currentScene && scene.sources.length > 0 ? true : get().isDirty,
    loadStatus: 'ready',
    pendingDuplicateCaptureSources: classification.duplicateCandidates,
    previewFramesBySceneSourceId: {},
    scene,
    selectedSceneSourceId:
      selectExistingSceneSource(scene, get().selectedSceneSourceId) ??
      scene.sources.at(-1)?.id ??
      null,
  });
}

export async function runStatusMutation(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
  action: RecorderLifecycleAction,
  mutation: () => Promise<RecorderStatus>,
): Promise<void> {
  await runAsyncStoreAction({
    before: () => set({ error: null, pendingLifecycleAction: action }),
    failure: (error) => {
      set({ error: errorMessage(error), pendingLifecycleAction: null });

      return isRecoverableRecorderActionError(error) ? false : undefined;
    },
    run: mutation,
    success: (status) => {
      if (!recorderStatusIsCurrent(get().status, status)) return;
      set({
        pendingLifecycleAction: statusShowsCommandInProgress(status)
          ? action
          : null,
        recordingStats:
          status.state === 'recording' ? get().recordingStats : null,
        recordingStatsError:
          status.state === 'recording' ? get().recordingStatsError : null,
        status,
      });
      if (statusBlocksLifecycle(status)) {
        void reconcileStatusUntilSettled(set, get, action);
      }
    },
  });
}

async function reconcileStatusUntilSettled(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
  action: RecorderLifecycleAction,
): Promise<void> {
  const deadline = Date.now() + FINALIZATION_POLL_TIMEOUT_MS;
  while (Date.now() < deadline) {
    await sleep(FINALIZATION_POLL_INTERVAL_MS);
    if (!statusBlocksLifecycle(get().status)) {
      return;
    }
    const status = await runAsyncStoreAction({
      failure: (error) => {
        set({
          error: errorMessage(error),
          pendingLifecycleAction: null,
        });
        return false;
      },
      run: () => getRecorderStatus(),
      success: (nextStatus) => {
        if (!recorderStatusIsCurrent(get().status, nextStatus)) return;
        set({
          error: nextStatus.lastError?.message ?? null,
          pendingLifecycleAction: statusShowsCommandInProgress(nextStatus)
            ? action
            : null,
          recordingStats:
            nextStatus.state === 'recording' ? get().recordingStats : null,
          recordingStatsError:
            nextStatus.state === 'recording' ? get().recordingStatsError : null,
          status: nextStatus,
        });
      },
    });
    if (!status || !statusBlocksLifecycle(get().status)) {
      return;
    }
  }
  set({
    error:
      'Recorder is still saving the recording. Diagnostics may show more detail.',
    pendingLifecycleAction: null,
  });
}
