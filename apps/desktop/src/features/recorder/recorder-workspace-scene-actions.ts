import type { AudioSourceRole, RecorderScene } from '@daydream/shared';
import {
  selectExistingSceneSource,
  workspaceConfigurationLocked,
} from './recorder-workspace-store-helpers';
import type {
  RecorderWorkspaceGet,
  RecorderWorkspaceSet,
  RecorderWorkspaceState,
} from './recorder-workspace-store-types';
import {
  removeSceneSource,
  resolveDuplicateCaptureSource as resolveDuplicateCaptureSourceInScene,
  type DuplicateCaptureSourceResolution,
} from './scene-layout';

type SceneActions = Pick<
  RecorderWorkspaceState,
  | 'deleteSceneSource'
  | 'resolveDuplicateCaptureSource'
  | 'selectAudioSource'
  | 'selectSceneSource'
  | 'setDraftScene'
>;

export function createRecorderWorkspaceSceneActions(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
): SceneActions {
  return {
    deleteSceneSource: (sceneSourceId) =>
      deleteSceneSource(set, get, sceneSourceId),
    resolveDuplicateCaptureSource: (resolution) =>
      resolveDuplicateCaptureSource(set, get, resolution),
    selectAudioSource: (role, sourceId) =>
      selectAudioSource(set, get, role, sourceId),
    selectSceneSource: (sourceId) => set({ selectedSceneSourceId: sourceId }),
    setDraftScene: (scene) => setDraftScene(set, get, scene),
  };
}

function deleteSceneSource(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
  sceneSourceId: string,
) {
  if (workspaceConfigurationLocked(get())) {
    return;
  }

  const scene = get().scene;
  if (!scene) {
    return;
  }

  const deletedSourceIndex = scene.sources.findIndex(
    (source) => source.id === sceneSourceId,
  );
  if (deletedSourceIndex === -1) {
    return;
  }

  const nextScene = removeSceneSource(scene, sceneSourceId);
  const nextSelectedSceneSourceId =
    get().selectedSceneSourceId === sceneSourceId
      ? (nextScene.sources[deletedSourceIndex]?.id ??
        nextScene.sources[deletedSourceIndex - 1]?.id ??
        null)
      : selectExistingSceneSource(nextScene, get().selectedSceneSourceId);

  set({
    error: null,
    isDirty: true,
    previewFramesBySceneSourceId: {},
    scene: nextScene,
    selectedSceneSourceId: nextSelectedSceneSourceId,
  });
}

function resolveDuplicateCaptureSource(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
  resolution: DuplicateCaptureSourceResolution,
) {
  if (workspaceConfigurationLocked(get())) {
    return;
  }

  const [candidate, ...remainingCandidates] =
    get().pendingDuplicateCaptureSources;
  const scene = get().scene;
  if (!candidate || !scene) {
    return;
  }

  const nextScene = resolveDuplicateCaptureSourceInScene(
    scene,
    candidate,
    resolution,
  );
  set({
    error: null,
    isDirty: resolution === 'skip' ? get().isDirty : true,
    pendingDuplicateCaptureSources: remainingCandidates,
    previewFramesBySceneSourceId: {},
    scene: nextScene,
    selectedSceneSourceId:
      resolution === 'refresh-existing'
        ? candidate.existingSource.id
        : (selectExistingSceneSource(nextScene, get().selectedSceneSourceId) ??
          nextScene.sources.at(-1)?.id ??
          null),
  });
}

function selectAudioSource(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
  role: AudioSourceRole,
  sourceId: string | null,
) {
  const state = get();
  if (workspaceConfigurationLocked(state)) {
    return;
  }

  if (role === 'desktop-audio') {
    if (state.desktopAudioSourceId === sourceId) {
      return;
    }

    set({
      desktopAudioSourceId: sourceId,
      isDirty: true,
      previewFramesBySceneSourceId: {},
      scene: state.scene
        ? { ...state.scene, desktopAudioSourceId: sourceId }
        : state.scene,
    });
    return;
  }

  if (state.microphoneAudioSourceId === sourceId) {
    return;
  }

  set({
    isDirty: true,
    microphoneAudioSourceId: sourceId,
    previewFramesBySceneSourceId: {},
    scene: state.scene
      ? { ...state.scene, microphoneAudioSourceId: sourceId }
      : state.scene,
  });
}

function setDraftScene(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
  scene: RecorderScene,
) {
  if (workspaceConfigurationLocked(get())) {
    return;
  }

  set({
    isDirty: true,
    previewFramesBySceneSourceId: {},
    scene,
    selectedSceneSourceId: selectExistingSceneSource(
      scene,
      get().selectedSceneSourceId,
    ),
  });
}
