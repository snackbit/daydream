import { runAsyncStoreAction } from '@/lib/async-store-action';
import {
  saveRecorderScene,
  selectCaptureSources,
  stopRecorderPreview,
} from './recorder-client';
import {
  errorMessage,
  isRecorderPortalSelectionCancelled,
} from './recorder-error-utils';
import {
  previewErrorStatus,
  previewIsActive,
  stopPreviewForRecording,
} from './recorder-preview-store';
import {
  applyLoadedRecorderWorkspace,
  applySelectedCaptureSources,
  loadRecorderWorkspaceSnapshot,
  selectExistingSceneSource,
  workspaceConfigurationLocked,
} from './recorder-workspace-store-helpers';
import type {
  RecorderWorkspaceGet,
  RecorderWorkspaceSet,
  RecorderWorkspaceState,
} from './recorder-workspace-store-types';

type PersistenceActions = Pick<
  RecorderWorkspaceState,
  'load' | 'saveScene' | 'updateCaptureSources'
>;

export function createRecorderWorkspacePersistenceActions(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
): PersistenceActions {
  return {
    load: () => load(set),
    saveScene: () => saveScene(set, get),
    updateCaptureSources: () => updateCaptureSources(set, get),
  };
}

async function load(set: RecorderWorkspaceSet) {
  await runAsyncStoreAction({
    before: () => set({ error: null, loadStatus: 'loading' }),
    failure: (error) =>
      set({
        error: errorMessage(error),
        loadStatus: 'error',
      }),
    run: loadRecorderWorkspaceSnapshot,
    success: (snapshot) => applyLoadedRecorderWorkspace(set, snapshot),
  });
}

async function saveScene(set: RecorderWorkspaceSet, get: RecorderWorkspaceGet) {
  if (workspaceConfigurationLocked(get())) {
    return;
  }

  const scene = get().scene;
  if (!scene) {
    set({ error: 'No recorder scene is available to save.' });
    return;
  }
  if (
    scene.sources.length === 0 &&
    previewIsActive(get().previewStatus) &&
    !(await stopPreviewForRecording(
      set,
      get().previewStatus?.sceneId ?? scene.id,
    ))
  ) {
    set({ saveStatus: 'error' });
    return;
  }

  await runAsyncStoreAction({
    before: () => set({ error: null, saveStatus: 'loading' }),
    failure: (error) =>
      set({
        error: errorMessage(error),
        saveStatus: 'error',
      }),
    run: () =>
      saveRecorderScene({
        scene: {
          ...scene,
          desktopAudioSourceId: get().desktopAudioSourceId,
          microphoneAudioSourceId: get().microphoneAudioSourceId,
        },
      }),
    success: (savedScene) => {
      const previewStatus = get().previewStatus;
      const clearPreviewError =
        previewStatus?.state === 'error' &&
        previewStatus.sceneId === savedScene.id;
      const clearPreviewState =
        savedScene.sources.length === 0 || clearPreviewError;
      set({
        desktopAudioSourceId: savedScene.desktopAudioSourceId,
        error: null,
        isDirty: false,
        microphoneAudioSourceId: savedScene.microphoneAudioSourceId,
        previewFramesBySceneSourceId: clearPreviewState
          ? {}
          : get().previewFramesBySceneSourceId,
        previewStartInFlightSceneId: clearPreviewState
          ? null
          : get().previewStartInFlightSceneId,
        previewStatus: clearPreviewState ? null : previewStatus,
        saveStatus: 'ready',
        scene: savedScene,
        selectedSceneSourceId: selectExistingSceneSource(
          savedScene,
          get().selectedSceneSourceId,
        ),
      });
    },
  });
}

async function updateCaptureSources(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
) {
  if (workspaceConfigurationLocked(get())) {
    return;
  }

  const previewStatus = get().previewStatus;
  if (previewStatus && previewIsActive(previewStatus)) {
    const previewSceneId = previewStatus.sceneId;
    try {
      const stoppedPreviewStatus = await stopRecorderPreview();
      set({
        previewFramesBySceneSourceId: {},
        previewStartInFlightSceneId: null,
        previewStatus: stoppedPreviewStatus,
      });
    } catch (error) {
      set({
        error: errorMessage(error),
        loadStatus: 'error',
        previewFramesBySceneSourceId: {},
        previewStartInFlightSceneId: null,
        previewStatus: previewErrorStatus(previewSceneId, error),
      });
      return;
    }
  }

  await runAsyncStoreAction({
    before: () => set({ error: null, loadStatus: 'loading' }),
    failure: (error) => {
      if (isRecorderPortalSelectionCancelled(error)) {
        set({ error: null, loadStatus: 'ready' });
        return false;
      }

      set({
        error: errorMessage(error),
        loadStatus: 'error',
      });
      return undefined;
    },
    run: () =>
      selectCaptureSources({
        forceChooser: true,
      }),
    success: (captureSourceList) =>
      applySelectedCaptureSources(set, get, captureSourceList.sources),
  });
}
