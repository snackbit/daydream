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
import { runAsyncStoreAction, sleep } from '@/lib/async-store-action';
import { sanitizeSceneAudioSources } from './recorder-audio-source-selection';
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
import {
  errorMessage,
  isRecorderPortalSelectionCancelled,
} from './recorder-error-utils';
import {
  isRecoverableRecorderActionError,
  recorderConfigurationLocked,
  recorderLifecycleLocked,
  statusBlocksLifecycle,
  statusShowsCommandInProgress,
} from './recorder-lifecycle-state';
import {
  previewErrorStatus,
  previewIsActive,
  stopPreviewForRecording,
} from './recorder-preview-store';
import {
  classifyCaptureSourceMerge,
  createDefaultRecorderScene,
  type DuplicateCaptureSourceCandidate,
  type DuplicateCaptureSourceResolution,
  mergeCaptureSourcesIntoScene,
  removeSceneSource,
  resolveDuplicateCaptureSource as resolveDuplicateCaptureSourceInScene,
} from './scene-layout';

type AsyncStatus = 'error' | 'idle' | 'loading' | 'ready';
export type RecorderLifecycleAction = 'pause' | 'resume' | 'start' | 'stop';

const FINALIZATION_POLL_INTERVAL_MS = 250;
const FINALIZATION_POLL_TIMEOUT_MS = 15_000;
export const DEFAULT_RECORDER_FPS = 5;

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
  previewStartInFlightSceneId: string | null;
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

type RecorderWorkspacePatch =
  | Partial<RecorderWorkspaceState>
  | ((state: RecorderWorkspaceState) => Partial<RecorderWorkspaceState>);
type RecorderWorkspaceSet = (partial: RecorderWorkspacePatch) => void;

export const useRecorderWorkspaceStore = create<RecorderWorkspaceState>(
  (set, get) => ({
    audioSources: [],
    captureSources: [],
    desktopAudioSourceId: null,
    error: null,
    isDirty: false,
    async load() {
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
    },
    deleteSceneSource(sceneSourceId) {
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
    },
    loadStatus: 'idle',
    microphoneAudioSourceId: null,
    pendingDuplicateCaptureSources: [],
    previewFramesBySceneSourceId: {},
    previewStartInFlightSceneId: null,
    previewStatus: null,
    async pause() {
      const recordingId = get().status?.activeRecordingId;
      if (!recordingId) {
        return;
      }

      if (recorderLifecycleLocked(get().pendingLifecycleAction, get().status)) {
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

      if (recorderLifecycleLocked(get().pendingLifecycleAction, get().status)) {
        return;
      }

      await runStatusMutation(set, get, 'resume', () =>
        resumeRecording({ recordingId }),
      );
    },
    resolveDuplicateCaptureSource(resolution) {
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
    },
    saveStatus: 'idle',
    scene: null,
    selectAudioSource(role, sourceId) {
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
    },
    selectSceneSource(sourceId) {
      set({ selectedSceneSourceId: sourceId });
    },
    selectedSceneSourceId: null,
    setDraftScene(scene) {
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
    },
    async start() {
      const state = get();
      const scene = state.scene;
      if (!scene || state.isDirty) {
        set({ error: 'Save the recorder scene before starting.' });
        return;
      }

      if (recorderLifecycleLocked(state.pendingLifecycleAction, state.status)) {
        return;
      }

      set({ error: null, pendingLifecycleAction: 'start' });

      if (
        previewIsActive(state.previewStatus) &&
        !(await stopPreviewForRecording(
          set,
          state.previewStatus?.sceneId ?? null,
        ))
      ) {
        set({ pendingLifecycleAction: null });
        return;
      }

      await runStatusMutation(set, get, 'start', () =>
        startRecording({
          desktopAudioSourceId: state.desktopAudioSourceId,
          fps: DEFAULT_RECORDER_FPS,
          microphoneAudioSourceId: state.microphoneAudioSourceId,
          qualityPreset: 'balanced-ocr',
          sceneId: scene.id,
        }),
      );
    },
    async startPreview(sceneId) {
      const state = get();
      if (
        recorderLifecycleLocked(state.pendingLifecycleAction, state.status) ||
        state.previewStartInFlightSceneId !== null ||
        (state.previewStatus?.sceneId === sceneId &&
          previewIsActive(state.previewStatus))
      ) {
        return;
      }

      await runAsyncStoreAction({
        before: () =>
          set({
            previewStartInFlightSceneId: sceneId,
            previewStatus: {
              error: null,
              sceneId,
              sourceIds: [],
              state: 'starting',
              updatedAtMs: Date.now(),
            },
          }),
        cleanup: () => {
          if (get().previewStartInFlightSceneId === sceneId) {
            set({ previewStartInFlightSceneId: null });
          }
        },
        failure: (error) => {
          if (get().previewStartInFlightSceneId !== sceneId) {
            return false;
          }

          set({
            previewFramesBySceneSourceId: {},
            previewStatus: previewErrorStatus(sceneId, error),
          });

          return undefined;
        },
        rethrow: false,
        run: () => startRecorderPreview({ sceneId }),
        success: (previewStatus) => {
          if (get().previewStartInFlightSceneId !== sceneId) return;
          if (previewStatus.sceneId !== sceneId) {
            set({
              previewFramesBySceneSourceId: {},
              previewStatus: previewErrorStatus(sceneId, null),
            });
            return;
          }

          set({ previewStatus });
        },
      });
    },
    status: null,
    async stop() {
      const recordingId = get().status?.activeRecordingId;
      if (!recordingId) {
        return;
      }

      if (recorderLifecycleLocked(get().pendingLifecycleAction, get().status)) {
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

      await runAsyncStoreAction({
        failure: (error) =>
          set({
            previewStartInFlightSceneId: null,
            previewStatus: previewErrorStatus(
              get().previewStatus?.sceneId,
              error,
            ),
          }),
        rethrow: false,
        run: () => stopRecorderPreview(),
        success: (previewStatus) => {
          set({
            previewFramesBySceneSourceId: {},
            previewStartInFlightSceneId: null,
            previewStatus,
          });
        },
      });
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
    },
  }),
);

function workspaceConfigurationLocked(state: RecorderWorkspaceState): boolean {
  return recorderConfigurationLocked(
    state.pendingLifecycleAction,
    state.status,
  );
}

function selectExistingSceneSource(
  scene: RecorderScene,
  sceneSourceId: string | null,
): string | null {
  return scene.sources.some((source) => source.id === sceneSourceId)
    ? sceneSourceId
    : (scene.sources[0]?.id ?? null);
}

async function loadRecorderWorkspaceSnapshot() {
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

  return {
    audioSourceList,
    captureSourceList,
    previewStatus,
    savedScene,
    status,
  };
}

function applyLoadedRecorderWorkspace(
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
}
function applySelectedCaptureSources(
  set: RecorderWorkspaceSet,
  get: () => RecorderWorkspaceState,
  sources: CaptureSource[],
): void {
  const currentScene = get().scene ?? createDefaultRecorderScene([]);
  const classification = classifyCaptureSourceMerge(currentScene, sources);
  const scene = mergeCaptureSourcesIntoScene(
    currentScene,
    classification.newSources,
  );
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
async function runStatusMutation(
  set: RecorderWorkspaceSet,
  get: () => RecorderWorkspaceState,
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
      set({
        pendingLifecycleAction: statusShowsCommandInProgress(status)
          ? action
          : null,
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
  get: () => RecorderWorkspaceState,
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
        set({
          error: nextStatus.lastError?.message ?? null,
          pendingLifecycleAction: statusShowsCommandInProgress(nextStatus)
            ? action
            : null,
          status: nextStatus,
        });
      },
    });
    if (!status || !statusBlocksLifecycle(status)) {
      return;
    }
  }
  set({
    error:
      'Recorder is still saving the recording. Diagnostics may show more detail.',
    pendingLifecycleAction: null,
  });
}
