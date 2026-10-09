import type { RecorderScene } from '@daydream/shared';
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
  'load' | 'reloadPreferences' | 'saveScene' | 'updateCaptureSources'
>;

export function createRecorderWorkspacePersistenceActions(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
): PersistenceActions {
  return {
    load: () => load(set, get),
    reloadPreferences: (savedScene) => load(set, get, savedScene),
    saveScene: () => saveScene(set, get),
    updateCaptureSources: () => updateCaptureSources(set, get),
  };
}

function revisionScopedSet(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
): RecorderWorkspaceSet {
  const revision = get().workspaceRevision;
  return (patch) => {
    if (revision === get().workspaceRevision) set(patch);
  };
}
async function load(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
  savedSceneOverride?: RecorderScene | null,
) {
  set = revisionScopedSet(set, get);
  const statusBeforeRead = get().status;
  await runAsyncStoreAction({
    before: () => set({ error: null, loadStatus: 'loading' }),
    failure: (error) =>
      set({
        error: errorMessage(error),
        loadStatus: 'error',
      }),
    run: () => loadRecorderWorkspaceSnapshot(savedSceneOverride),
    success: (snapshot) =>
      applyLoadedRecorderWorkspace(set, {
        ...snapshot,
        status:
          get().status !== statusBeforeRead
            ? (get().status ?? snapshot.status)
            : snapshot.status,
      }),
  });
}

async function saveScene(set: RecorderWorkspaceSet, get: RecorderWorkspaceGet) {
  const revision = get().workspaceRevision;
  set = revisionScopedSet(set, get);
  if (workspaceConfigurationLocked(get()) || get().saveStatus === 'loading') {
    return;
  }

  const scene = get().scene;
  if (!scene) {
    set({ error: 'No recorder scene is available to save.' });
    return;
  }
  const desktopAudioSourceId = get().desktopAudioSourceId;
  const microphoneAudioSourceId = get().microphoneAudioSourceId;
  set({ error: null, saveStatus: 'loading' });
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

  if (revision !== get().workspaceRevision) return;
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
          desktopAudioSourceId,
          microphoneAudioSourceId,
        },
      }),
    success: (savedScene) => {
      const draftChanged =
        get().scene !== scene ||
        get().desktopAudioSourceId !== desktopAudioSourceId ||
        get().microphoneAudioSourceId !== microphoneAudioSourceId;
      const currentScene = draftChanged ? get().scene : savedScene;
      const previewStatus = get().previewStatus;
      const clearPreviewError =
        previewStatus?.state === 'error' &&
        previewStatus.sceneId === savedScene.id;
      const clearPreviewState =
        savedScene.sources.length === 0 || clearPreviewError;
      set({
        desktopAudioSourceId: draftChanged
          ? get().desktopAudioSourceId
          : savedScene.desktopAudioSourceId,
        error: null,
        isDirty: draftChanged,
        microphoneAudioSourceId: draftChanged
          ? get().microphoneAudioSourceId
          : savedScene.microphoneAudioSourceId,
        previewFramesBySceneSourceId: clearPreviewState
          ? {}
          : get().previewFramesBySceneSourceId,
        previewStartInFlightSceneId: clearPreviewState
          ? null
          : get().previewStartInFlightSceneId,
        previewStatus: clearPreviewState ? null : previewStatus,
        saveStatus: 'ready',
        scene: currentScene,
        selectedSceneSourceId: currentScene
          ? selectExistingSceneSource(currentScene, get().selectedSceneSourceId)
          : null,
      });
    },
  });
}

async function updateCaptureSources(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
) {
  const revision = get().workspaceRevision;
  set = revisionScopedSet(set, get);
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

  if (revision !== get().workspaceRevision) return;
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
