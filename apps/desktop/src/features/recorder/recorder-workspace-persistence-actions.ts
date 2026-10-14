import type { RecorderScene } from '@daydream/shared';
import { runAsyncStoreAction } from '@/lib/async-store-action';
import { saveRecorderScene, selectCaptureSources } from './recorder-client';
import {
  errorMessage,
  isRecorderPortalSelectionCancelled,
} from './recorder-error-utils';
import {
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
    !(await stopPreviewForRecording(set, get))
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
  if (workspaceConfigurationLocked(get()) || get().captureSelectionPending) {
    return;
  }
  // Own the entire selection, including cleanup, before any awaited IPC.
  set({ captureSelectionPending: true, error: null, loadStatus: 'loading' });
  await runAsyncStoreAction({
    cleanup: () => set({ captureSelectionPending: false }),
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
    run: async () => {
      const needsStop = previewIsActive(get().previewStatus);
      await get().stopPreview();
      if (revision !== get().workspaceRevision) return null;
      if (needsStop && get().previewStatus?.state === 'error') {
        throw (
          get().previewStatus?.error ??
          new Error('Preview could not stop. Try again.')
        );
      }
      return selectCaptureSources({ forceChooser: true });
    },
    success: (captureSourceList) => {
      if (captureSourceList)
        applySelectedCaptureSources(set, get, captureSourceList.sources);
    },
  });
}
