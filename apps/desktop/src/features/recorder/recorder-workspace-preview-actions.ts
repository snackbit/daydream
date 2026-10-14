import { runAsyncStoreAction } from '@/lib/async-store-action';
import {
  listenToRecorderPreviewFrames,
  startRecorderPreview,
  stopRecorderPreview,
} from './recorder-client';
import { errorMessage } from './recorder-error-utils';
import { recorderLifecycleLocked } from './recorder-lifecycle-state';
import { previewErrorStatus, previewIsActive } from './recorder-preview-store';
import type {
  RecorderWorkspaceGet,
  RecorderWorkspaceSet,
  RecorderWorkspaceState,
} from './recorder-workspace-store-types';

type PreviewActions = Pick<
  RecorderWorkspaceState,
  'startPreview' | 'stopPreview' | 'subscribeToPreviewFrames'
>;

export function createRecorderWorkspacePreviewActions(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
): PreviewActions {
  let pendingStop: Promise<void> | null = null;
  function stopCurrentPreview() {
    if (pendingStop && get().previewStopInFlight) return pendingStop;
    const operation = stopPreview(set, get).finally(() => {
      if (pendingStop === operation) pendingStop = null;
    });
    pendingStop = operation;
    return operation;
  }
  return {
    startPreview: (sceneId) => startPreview(set, get, sceneId),
    stopPreview: stopCurrentPreview,
    subscribeToPreviewFrames: () => subscribeToPreviewFrames(set),
  };
}

async function startPreview(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
  sceneId: string,
) {
  const state = get();
  if (
    recorderLifecycleLocked(state.pendingLifecycleAction, state.status) ||
    state.captureSelectionPending ||
    state.previewStopInFlight ||
    state.previewStartInFlightSceneId !== null ||
    (state.previewStatus?.sceneId === sceneId &&
      previewIsActive(state.previewStatus))
  ) {
    return;
  }

  const generation = state.previewOperationGeneration + 1;
  const revision = state.workspaceRevision;
  const current = () =>
    get().previewOperationGeneration === generation &&
    get().workspaceRevision === revision;
  await runAsyncStoreAction({
    before: () =>
      set({
        previewStartInFlightSceneId: sceneId,
        previewOperationGeneration: generation,
        previewStatus: {
          error: null,
          sceneId,
          sourceIds: [],
          state: 'starting',
          updatedAtMs: Date.now(),
        },
      }),
    cleanup: () => {
      if (current()) {
        set({ previewStartInFlightSceneId: null });
      }
    },
    failure: (error) => {
      if (!current()) {
        return false;
      }

      set({
        previewFramesBySceneSourceId: {},
        previewStatus: previewErrorStatus(sceneId, error),
      });

      return undefined;
    },
    rethrow: false,
    run: () =>
      startRecorderPreview({
        sceneId,
        ...(state.scene && state.isDirty ? { scene: state.scene } : {}),
      }),
    success: (previewStatus) => {
      if (!current()) return;
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
}

async function stopPreview(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
) {
  const state = get();
  if (state.previewStopInFlight || !previewIsActive(state.previewStatus)) {
    return;
  }

  const generation = state.previewOperationGeneration + 1;
  const revision = state.workspaceRevision;
  const current = () =>
    get().previewOperationGeneration === generation &&
    get().workspaceRevision === revision;
  await runAsyncStoreAction({
    before: () =>
      set({
        previewOperationGeneration: generation,
        previewStopInFlight: true,
        previewStartInFlightSceneId: null,
      }),
    cleanup: () => {
      if (current()) set({ previewStopInFlight: false });
    },
    failure: (error) => {
      if (current())
        set({
          error: errorMessage(error),
          previewStatus: previewErrorStatus(
            state.previewStatus?.sceneId,
            error,
          ),
        });
    },
    rethrow: false,
    run: () => stopRecorderPreview(),
    success: (previewStatus) => {
      if (!current()) return;
      set({
        previewFramesBySceneSourceId: {},
        previewStartInFlightSceneId: null,
        previewStatus,
      });
    },
  });
}

async function subscribeToPreviewFrames(set: RecorderWorkspaceSet) {
  return listenToRecorderPreviewFrames((frame) => {
    set((state) =>
      (state.scene
        ? state.scene.sources.some(
            (source) => source.id === frame.sceneSourceId,
          )
        : state.previewStatus?.state === 'live' &&
          state.previewStatus.sourceIds.includes(frame.sourceId)) &&
      (frame.generation === undefined ||
        state.previewFramesBySceneSourceId[frame.sceneSourceId]?.generation ===
          undefined ||
        frame.generation >
          (state.previewFramesBySceneSourceId[frame.sceneSourceId]
            ?.generation ?? 0) ||
        (frame.generation ===
          state.previewFramesBySceneSourceId[frame.sceneSourceId]?.generation &&
          (frame.sequence ?? 0) >
            (state.previewFramesBySceneSourceId[frame.sceneSourceId]
              ?.sequence ?? 0)))
        ? {
            previewFramesBySceneSourceId: {
              ...state.previewFramesBySceneSourceId,
              [frame.sceneSourceId]: frame,
            },
          }
        : {},
    );
  });
}
