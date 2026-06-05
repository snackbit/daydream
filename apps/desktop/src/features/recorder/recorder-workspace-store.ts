import type {
  AudioSource,
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
  pauseRecording,
  resumeRecording,
  saveRecorderScene,
  selectCaptureSources,
  startRecording,
  stopRecording,
} from './recorder-client';
import {
  createDefaultRecorderScene,
  mergeCaptureSourcesIntoScene,
  removeSceneSource,
} from './scene-layout';

type AsyncStatus = 'error' | 'idle' | 'loading' | 'ready';

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
  resume: () => Promise<void>;
  saveScene: () => Promise<void>;
  saveStatus: AsyncStatus;
  scene: RecorderScene | null;
  selectAudioSource: (
    role: 'desktop-audio' | 'microphone',
    sourceId: string | null,
  ) => void;
  selectSceneSource: (sourceId: string | null) => void;
  selectedSceneSourceId: string | null;
  setDraftScene: (scene: RecorderScene) => void;
  start: () => Promise<void>;
  status: RecorderStatus | null;
  stop: () => Promise<void>;
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
        const scene =
          savedScene ?? createDefaultRecorderScene(captureSourceList.sources);

        set({
          audioSources: audioSourceList.sources,
          captureSources: captureSourceList.sources,
          desktopAudioSourceId: defaultAudioSourceId(
            audioSourceList.sources,
            'desktop-audio',
          ),
          error: null,
          isDirty: !savedScene && captureSourceList.sources.length > 0,
          loadStatus: 'ready',
          microphoneAudioSourceId: defaultAudioSourceId(
            audioSourceList.sources,
            'microphone',
          ),
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
    async pause() {
      const recordingId = get().status?.activeRecordingId;
      if (!recordingId) {
        return;
      }

      await runStatusMutation(set, () => pauseRecording({ recordingId }));
    },
    async resume() {
      const recordingId = get().status?.activeRecordingId;
      if (!recordingId) {
        return;
      }

      await runStatusMutation(set, () => resumeRecording({ recordingId }));
    },
    async saveScene() {
      const scene = get().scene;
      if (!scene) {
        set({ error: 'No recorder scene is available to save.' });
        return;
      }

      set({ error: null, saveStatus: 'loading' });

      try {
        const savedScene = await saveRecorderScene({ scene });
        set({
          error: null,
          isDirty: false,
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
      if (role === 'desktop-audio') {
        set({ desktopAudioSourceId: sourceId });
        return;
      }

      set({ microphoneAudioSourceId: sourceId });
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

      await runStatusMutation(set, () =>
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

      await runStatusMutation(set, () => stopRecording({ recordingId }));
    },
    async updateCaptureSources() {
      set({ error: null, loadStatus: 'loading' });

      try {
        const captureSourceList = await selectCaptureSources({
          forceChooser: true,
        });
        const currentScene = get().scene ?? createDefaultRecorderScene([]);
        const scene = mergeCaptureSourcesIntoScene(
          currentScene,
          captureSourceList.sources,
        );
        set({
          captureSources: captureSourceList.sources,
          error: null,
          isDirty: scene.sources.length > 0,
          loadStatus: 'ready',
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

function defaultAudioSourceId(
  sources: AudioSource[],
  role: 'desktop-audio' | 'microphone',
): string | null {
  return (
    sources.find((source) => source.isDefault && source.roles.includes(role))
      ?.id ??
    sources.find((source) => source.roles.includes(role))?.id ??
    null
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

async function runStatusMutation(
  set: (
    partial:
      | Partial<RecorderWorkspaceState>
      | ((state: RecorderWorkspaceState) => Partial<RecorderWorkspaceState>),
  ) => void,
  mutation: () => Promise<RecorderStatus>,
): Promise<void> {
  set({ error: null });

  try {
    const status = await mutation();
    set({ status });
  } catch (error) {
    set({ error: errorMessage(error) });
  }
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
