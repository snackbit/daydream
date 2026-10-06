import { runAsyncStoreAction } from '@/lib/async-store-action';
import {
  listenToRecorderPreviewFrames,
  startRecorderPreview,
  stopRecorderPreview,
} from './recorder-client';
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
  return {
    startPreview: (sceneId) => startPreview(set, get, sceneId),
    stopPreview: () => stopPreview(set, get),
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
    run: () =>
      startRecorderPreview({
        sceneId,
        ...(state.scene && state.isDirty ? { scene: state.scene } : {}),
      }),
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
}

async function stopPreview(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
) {
  if (!previewIsActive(get().previewStatus)) {
    return;
  }

  await runAsyncStoreAction({
    failure: (error) =>
      set({
        previewStartInFlightSceneId: null,
        previewStatus: previewErrorStatus(get().previewStatus?.sceneId, error),
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
}

async function subscribeToPreviewFrames(set: RecorderWorkspaceSet) {
  return listenToRecorderPreviewFrames((frame) => {
    set((state) => ({
      previewFramesBySceneSourceId: {
        ...state.previewFramesBySceneSourceId,
        [frame.sceneSourceId]: frame,
      },
    }));
  });
}
