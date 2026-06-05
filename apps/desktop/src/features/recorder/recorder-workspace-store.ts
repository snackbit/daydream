import type {
  AudioSource,
  AudioSourceRole,
  CaptureSource,
  RecorderPreviewFrame,
  RecorderPreviewStatus,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import { create } from 'zustand';
import {
  getAudioSources,
  getCaptureSources,
  getRecorderPreviewStatus,
  getRecorderScene,
  getRecorderStatus,
  listenToRecorderPreviewFrames,
  listenToRecorderStatusChanged,
  pauseRecording,
  resumeRecording,
  saveRecorderScene,
  selectCaptureSources,
  startRecorderPreview,
  startRecording,
  stopRecorderPreview,
  stopRecording,
} from './recorder-client';
import { errorMessage } from './recorder-error-utils';
import {
  previewErrorStatus,
  previewIsActive,
  stopPreviewForRecording,
} from './recorder-preview-store';
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

const FINALIZATION_POLL_INTERVAL_MS = 250;
const FINALIZATION_POLL_TIMEOUT_MS = 15_000;

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
  previewFramesBySceneSourceId: Record<string, RecorderPreviewFrame>;
  previewStatus: RecorderPreviewStatus | null;
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
  startPreview: (sceneId: string) => Promise<void>;
  status: RecorderStatus | null;
  stop: () => Promise<void>;
  stopPreview: () => Promise<void>;
  subscribeToPreviewFrames: () => Promise<() => void>;
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
          previewFramesBySceneSourceId: {},
          previewStatus,
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
        previewFramesBySceneSourceId: {},
        scene: nextScene,
        selectedSceneSourceId: nextSelectedSceneSourceId,
      });
    },
    loadStatus: 'idle',
    microphoneAudioSourceId: null,
    pendingDuplicateCaptureSources: [],
    previewFramesBySceneSourceId: {},
    previewStatus: null,
    async pause() {
      const recordingId = get().status?.activeRecordingId;
      if (!recordingId) {
        return;
      }

      if (recorderLifecycleLocked(get())) {
        return;
      }

      await runStatusMutation(set, get, 'pause', () =>
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

      await runStatusMutation(set, get, 'resume', () =>
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
        previewFramesBySceneSourceId: {},
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
    },
    selectSceneSource(sourceId) {
      set({ selectedSceneSourceId: sourceId });
    },
    selectedSceneSourceId: null,
    setDraftScene(scene) {
      set({
        isDirty: true,
        previewFramesBySceneSourceId: {},
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

      if (
        previewIsActive(state.previewStatus) &&
        !(await stopPreviewForRecording(
          set,
          state.previewStatus?.sceneId ?? null,
        ))
      ) {
        return;
      }

      await runStatusMutation(set, get, 'start', () =>
        startRecording({
          desktopAudioSourceId: state.desktopAudioSourceId,
          fps: 1,
          microphoneAudioSourceId: state.microphoneAudioSourceId,
          qualityPreset: 'balanced-ocr',
          sceneId: scene.id,
        }),
      );
    },
    async startPreview(sceneId) {
      const state = get();
      if (
        state.previewStatus?.sceneId === sceneId &&
        (state.previewStatus.state === 'starting' ||
          state.previewStatus.state === 'live')
      ) {
        return;
      }

      set({
        previewStatus: {
          error: null,
          sceneId,
          sourceIds: [],
          state: 'starting',
          updatedAtMs: Date.now(),
        },
      });

      try {
        const previewStatus = await startRecorderPreview({ sceneId });
        if (previewStatus.sceneId !== sceneId) {
          set({
            previewFramesBySceneSourceId: {},
            previewStatus: previewErrorStatus(sceneId, null),
          });
          return;
        }

        set({ previewStatus });
      } catch (error) {
        set({
          previewFramesBySceneSourceId: {},
          previewStatus: previewErrorStatus(sceneId, error),
        });
      }
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

      await runStatusMutation(set, get, 'stop', () =>
        stopRecording({ recordingId }),
      );
    },
    async stopPreview() {
      if (!previewIsActive(get().previewStatus)) {
        return;
      }

      try {
        const previewStatus = await stopRecorderPreview();
        set({
          previewFramesBySceneSourceId: {},
          previewStatus,
        });
      } catch (error) {
        set({
          previewStatus: previewErrorStatus(
            get().previewStatus?.sceneId,
            error,
          ),
        });
      }
    },
    async subscribeToPreviewFrames() {
      return listenToRecorderPreviewFrames((frame) => {
        set((state) => ({
          previewFramesBySceneSourceId: {
            ...state.previewFramesBySceneSourceId,
            [frame.sceneSourceId]: frame,
          },
        }));
      });
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
          previewFramesBySceneSourceId: {},
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
  get: () => RecorderWorkspaceState,
  action: RecorderLifecycleAction,
  mutation: () => Promise<RecorderStatus>,
): Promise<void> {
  set({ error: null, pendingLifecycleAction: action });

  try {
    const status = await mutation();
    set({
      pendingLifecycleAction: statusShowsCommandInProgress(status)
        ? action
        : null,
      status,
    });
    if (statusNeedsReconciliation(status)) {
      void reconcileStatusUntilSettled(set, get, action);
    }
  } catch (error) {
    set({ error: errorMessage(error), pendingLifecycleAction: null });
  }
}

function recorderLifecycleLocked(state: RecorderWorkspaceState): boolean {
  return (
    state.pendingLifecycleAction !== null || statusBlocksLifecycle(state.status)
  );
}

function statusBlocksLifecycle(status: RecorderStatus | null): boolean {
  return status?.finalization != null || statusShowsCommandInProgress(status);
}

function statusShowsCommandInProgress(status: RecorderStatus | null): boolean {
  return (
    status?.state === 'pausing' ||
    status?.state === 'selecting' ||
    status?.state === 'stopping'
  );
}

function statusNeedsReconciliation(status: RecorderStatus | null): boolean {
  return statusBlocksLifecycle(status);
}

async function reconcileStatusUntilSettled(
  set: (
    partial:
      | Partial<RecorderWorkspaceState>
      | ((state: RecorderWorkspaceState) => Partial<RecorderWorkspaceState>),
  ) => void,
  get: () => RecorderWorkspaceState,
  action: RecorderLifecycleAction,
): Promise<void> {
  const deadline = Date.now() + FINALIZATION_POLL_TIMEOUT_MS;

  while (Date.now() < deadline) {
    await sleep(FINALIZATION_POLL_INTERVAL_MS);

    if (!statusNeedsReconciliation(get().status)) {
      return;
    }

    try {
      const status = await getRecorderStatus();
      set({
        error: status.lastError?.message ?? null,
        pendingLifecycleAction: statusShowsCommandInProgress(status)
          ? action
          : null,
        status,
      });

      if (!statusNeedsReconciliation(status)) {
        return;
      }
    } catch (error) {
      set({
        error: errorMessage(error),
        pendingLifecycleAction: null,
      });
      return;
    }
  }

  set({
    error:
      'Recorder is still saving the recording. Diagnostics may show more detail.',
    pendingLifecycleAction: null,
  });
}

function sleep(durationMs: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}
