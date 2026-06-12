import {
  getRecorderRecordingStats,
  listenToRecorderStatusChanged,
  pauseRecording,
  resumeRecording,
  startRecording,
  stopRecording,
} from './recorder-client';
import { errorMessage } from './recorder-error-utils';
import { recorderLifecycleLocked } from './recorder-lifecycle-state';
import {
  previewIsActive,
  stopPreviewForRecording,
} from './recorder-preview-store';
import { runStatusMutation } from './recorder-workspace-store-helpers';
import type {
  RecorderWorkspaceGet,
  RecorderWorkspaceSet,
  RecorderWorkspaceState,
} from './recorder-workspace-store-types';

type LifecycleActions = Pick<
  RecorderWorkspaceState,
  | 'pause'
  | 'refreshRecordingStats'
  | 'resume'
  | 'start'
  | 'stop'
  | 'subscribeToStatusEvents'
>;

export function createRecorderWorkspaceLifecycleActions(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
): LifecycleActions {
  return {
    pause: () => pause(set, get),
    refreshRecordingStats: () => refreshRecordingStats(set, get),
    resume: () => resume(set, get),
    start: () => start(set, get),
    stop: () => stop(set, get),
    subscribeToStatusEvents: () => subscribeToStatusEvents(set, get),
  };
}

async function pause(set: RecorderWorkspaceSet, get: RecorderWorkspaceGet) {
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
}

async function resume(set: RecorderWorkspaceSet, get: RecorderWorkspaceGet) {
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
}

async function refreshRecordingStats(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
) {
  if (get().status?.state !== 'recording') {
    set({ recordingStats: null, recordingStatsError: null });
    return;
  }

  try {
    const recordingStats = await getRecorderRecordingStats();
    const status = get().status;
    if (
      status?.state !== 'recording' ||
      (recordingStats !== null &&
        status.activeRecordingId !== recordingStats.recordingId)
    ) {
      set({ recordingStats: null, recordingStatsError: null });
      return;
    }

    set({ recordingStats, recordingStatsError: null });
  } catch (error) {
    set({ recordingStatsError: errorMessage(error) });
  }
}

async function start(set: RecorderWorkspaceSet, get: RecorderWorkspaceGet) {
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
    !(await stopPreviewForRecording(set, state.previewStatus?.sceneId ?? null))
  ) {
    set({ pendingLifecycleAction: null });
    return;
  }

  await runStatusMutation(set, get, 'start', () =>
    startRecording({
      desktopAudioSourceId: state.desktopAudioSourceId,
      microphoneAudioSourceId: state.microphoneAudioSourceId,
      qualityPreset: 'balanced-ocr',
      sceneId: scene.id,
    }),
  );
}

async function stop(set: RecorderWorkspaceSet, get: RecorderWorkspaceGet) {
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
}

async function subscribeToStatusEvents(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
) {
  return listenToRecorderStatusChanged((status) => {
    set({
      error: status.lastError?.message ?? null,
      pendingLifecycleAction: null,
      recordingStats:
        status.state === 'recording' ? get().recordingStats : null,
      recordingStatsError:
        status.state === 'recording' ? get().recordingStatsError : null,
      status,
    });
  });
}
