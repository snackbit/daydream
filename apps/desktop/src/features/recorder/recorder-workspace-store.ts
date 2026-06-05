import type {
  AudioSource,
  AudioSourceRole,
  CaptureSource,
  RecorderError,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import { create } from 'zustand';
import {
  getAudioSources,
  getCaptureSources,
  getRecorderScene,
  getRecorderStatus,
  listenToRecorderStatusChanged,
  pauseRecording,
  resumeRecording,
  saveRecorderScene,
  selectCaptureSources,
  startRecording,
  stopRecording,
} from './recorder-client';
import {
  classifyCaptureSourceMerge,
  createDefaultRecorderScene,
  mergeCaptureSourcesIntoScene,
  removeSceneSource,
  resolveDuplicateCaptureSource as resolveDuplicateCaptureSourceInScene,
} from './scene-layout';
import type {
  DuplicateCaptureSourceCandidate,
  DuplicateCaptureSourceResolution,
} from './scene-layout';

type AsyncStatus = 'error' | 'idle' | 'loading' | 'ready';
export type RecorderLifecycleAction = 'pause' | 'resume' | 'start' | 'stop';

interface RecorderWorkspaceState {
  audioSources: AudioSource[];
  captureSources: CaptureSource[];
  desktopAudioSourceId: string | null;
  error: string | null;
  isDirty: boolean;
  deleteSceneSource: (sceneSourceId: string) => void;
  load: () => Promise<void>;
  loadStatus: AsyncStatus;
  microphoneAudioSourceId: string | null;
  pause: () => Promise<void>;
  pendingLifecycleAction: RecorderLifecycleAction | null;
  pendingDuplicateCaptureSources: DuplicateCaptureSourceCandidate[];
  resume: () => Promise<void>;
  resolveDuplicateCaptureSource: (
    resolution: DuplicateCaptureSourceResolution,
  ) => void;
  saveScene: () => Promise<void>;
  saveStatus: AsyncStatus;
  scene: RecorderScene | null;
  selectAudioSource: (role: AudioSourceRole, sourceId: string | null) => void;
  selectSceneSource: (sourceId: string | null) => void;
  selectedSceneSourceId: string | null;
  setDraftScene: (scene: RecorderScene) => void;
  start: () => Promise<void>;
  status: RecorderStatus | null;
  stop: () => Promise<void>;
  subscribeToStatusEvents: () => Promise<() => void>;
  updateCaptureSources: () => Promise<void>;
}

export const useRecorderWorkspaceStore = create<RecorderWorkspaceState>(
  (set, get) => ({
    audioSources: [],
    captureSources: [],
    desktopAudioSourceId: null,
    error: null,
    isDirty: false,
    async load() {
      set({ error: null, loadStatus: 'loading' });

      try {
        const [status, captureSourceList, audioSourceList, savedScene] =
          await Promise.all([
            getRecorderStatus(),
            getCaptureSources(),
            getAudioSources(),
            getRecorderScene(),
          ]);
        const restoredScene =
          savedScene ?? createDefaultRecorderScene(captureSourceList.sources);
        const scene = sanitizeSceneAudioSources(
          restoredScene,
          audioSourceList.sources,
        );
        const droppedSavedAudioSource =
          !!savedScene &&
          (scene.desktopAudioSourceId !== savedScene.desktopAudioSourceId ||
            scene.microphoneAudioSourceId !==
              savedScene.microphoneAudioSourceId);

        set({
          audioSources: audioSourceList.sources,
          captureSources: captureSourceList.sources,
          desktopAudioSourceId: scene.desktopAudioSourceId,
          error: null,
          isDirty:
            droppedSavedAudioSource ||
            (!savedScene && captureSourceList.sources.length > 0),
          loadStatus: 'ready',
          microphoneAudioSourceId: scene.microphoneAudioSourceId,
          scene,
          selectedSceneSourceId: scene.sources[0]?.id ?? null,
          status,
        });
      } catch (error) {
        set({
          error: errorMessage(error),
          loadStatus: 'error',
        });
      }
    },
    deleteSceneSource(sceneSourceId) {
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
        scene: nextScene,
        selectedSceneSourceId: nextSelectedSceneSourceId,
      });
    },
    loadStatus: 'idle',
    microphoneAudioSourceId: null,
    pendingDuplicateCaptureSources: [],
    async pause() {
      const recordingId = get().status?.activeRecordingId;
      if (!recordingId) {
        return;
      }

      if (recorderLifecycleLocked(get())) {
        return;
      }

      await runStatusMutation(set, 'pause', () =>
        pauseRecording({ recordingId }),
      );
    },
    async resume() {
      const recordingId = get().status?.activeRecordingId;
      if (!recordingId) {
        return;
      }

      if (recorderLifecycleLocked(get())) {
        return;
      }

      await runStatusMutation(set, 'resume', () =>
        resumeRecording({ recordingId }),
      );
    },
    resolveDuplicateCaptureSource(resolution) {
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
        scene: nextScene,
        selectedSceneSourceId:
          resolution === 'refresh-existing'
            ? candidate.existingSource.id
            : (selectExistingSceneSource(
                nextScene,
                get().selectedSceneSourceId,
              ) ??
              nextScene.sources.at(-1)?.id ??
              null),
      });
    },
    pendingLifecycleAction: null,
    async saveScene() {
      const scene = get().scene;
      if (!scene) {
        set({ error: 'No recorder scene is available to save.' });
        return;
      }

      set({ error: null, saveStatus: 'loading' });

      try {
        const sceneWithAudioSources = {
          ...scene,
          desktopAudioSourceId: get().desktopAudioSourceId,
          microphoneAudioSourceId: get().microphoneAudioSourceId,
        };
        const savedScene = await saveRecorderScene({
          scene: sceneWithAudioSources,
        });
        set({
          desktopAudioSourceId: savedScene.desktopAudioSourceId,
          error: null,
          isDirty: false,
          microphoneAudioSourceId: savedScene.microphoneAudioSourceId,
          saveStatus: 'ready',
          scene: savedScene,
          selectedSceneSourceId: selectExistingSceneSource(
            savedScene,
            get().selectedSceneSourceId,
          ),
        });
      } catch (error) {
        set({
          error: errorMessage(error),
          saveStatus: 'error',
        });
      }
    },
    saveStatus: 'idle',
    scene: null,
    selectAudioSource(role, sourceId) {
      const state = get();
      if (role === 'desktop-audio') {
        if (state.desktopAudioSourceId === sourceId) {
          return;
        }

        set({
          desktopAudioSourceId: sourceId,
          isDirty: true,
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
        scene: state.scene
          ? { ...state.scene, microphoneAudioSourceId: sourceId }
          : state.scene,
      });
    },
    selectSceneSource(sourceId) {
      set({ selectedSceneSourceId: sourceId });
    },
    selectedSceneSourceId: null,
    setDraftScene(scene) {
      set({
        isDirty: true,
        scene,
        selectedSceneSourceId: selectExistingSceneSource(
          scene,
          get().selectedSceneSourceId,
        ),
      });
    },
    async start() {
      const state = get();
      const scene = state.scene;
      if (!scene || state.isDirty) {
        set({ error: 'Save the recorder scene before starting.' });
        return;
      }

      if (recorderLifecycleLocked(state)) {
        return;
      }

      await runStatusMutation(set, 'start', () =>
        startRecording({
          desktopAudioSourceId: state.desktopAudioSourceId,
          fps: 1,
          microphoneAudioSourceId: state.microphoneAudioSourceId,
          qualityPreset: 'balanced-ocr',
          sceneId: scene.id,
        }),
      );
    },
    status: null,
    async stop() {
      const recordingId = get().status?.activeRecordingId;
      if (!recordingId) {
        return;
      }

      if (recorderLifecycleLocked(get())) {
        return;
      }

      await runStatusMutation(set, 'stop', () =>
        stopRecording({ recordingId }),
      );
    },
    async subscribeToStatusEvents() {
      return listenToRecorderStatusChanged((status) => {
        set({
          error: status.lastError?.message ?? null,
          pendingLifecycleAction: null,
          status,
        });
      });
    },
    async updateCaptureSources() {
      set({ error: null, loadStatus: 'loading' });

      try {
        const captureSourceList = await selectCaptureSources({
          forceChooser: true,
        });
        const currentScene = get().scene ?? createDefaultRecorderScene([]);
        const classification = classifyCaptureSourceMerge(
          currentScene,
          captureSourceList.sources,
        );
        const scene = mergeCaptureSourcesIntoScene(
          currentScene,
          classification.newSources,
        );
        set({
          captureSources: captureSourceList.sources,
          error: null,
          isDirty:
            scene !== currentScene && scene.sources.length > 0
              ? true
              : get().isDirty,
          loadStatus: 'ready',
          pendingDuplicateCaptureSources: classification.duplicateCandidates,
          scene,
          selectedSceneSourceId:
            selectExistingSceneSource(scene, get().selectedSceneSourceId) ??
            scene.sources.at(-1)?.id ??
            null,
        });
      } catch (error) {
        set({
          error: errorMessage(error),
          loadStatus: 'error',
        });
      }
    },
  }),
);

function selectExistingSceneSource(
  scene: RecorderScene,
  sceneSourceId: string | null,
): string | null {
  return scene.sources.some((source) => source.id === sceneSourceId)
    ? sceneSourceId
    : (scene.sources[0]?.id ?? null);
}

function sanitizeSceneAudioSources(
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

async function runStatusMutation(
  set: (
    partial:
      | Partial<RecorderWorkspaceState>
      | ((state: RecorderWorkspaceState) => Partial<RecorderWorkspaceState>),
  ) => void,
  action: RecorderLifecycleAction,
  mutation: () => Promise<RecorderStatus>,
): Promise<void> {
  set({ error: null, pendingLifecycleAction: action });

  try {
    const status = await mutation();
    set({
      pendingLifecycleAction: statusNeedsCompletionEvent(status)
        ? action
        : null,
      status,
    });
  } catch (error) {
    set({ error: errorMessage(error), pendingLifecycleAction: null });
  }
}

function recorderLifecycleLocked(state: RecorderWorkspaceState): boolean {
  return (
    state.pendingLifecycleAction !== null ||
    statusNeedsCompletionEvent(state.status)
  );
}

function statusNeedsCompletionEvent(status: RecorderStatus | null): boolean {
  return (
    status?.state === 'pausing' ||
    status?.state === 'selecting' ||
    status?.state === 'stopping'
  );
}

function errorMessage(error: unknown): string {
  if (isRecorderError(error)) {
    return error.message;
  }

  return error instanceof Error ? error.message : String(error);
}

function isRecorderError(error: unknown): error is RecorderError {
  return (
    typeof error === 'object' &&
    error !== null &&
    'category' in error &&
    'code' in error &&
    'message' in error
  );
}
