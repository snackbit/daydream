import type { SaveCaptureSetupInput } from '@daydream/shared';
import {
  getRecorderRecordingStats,
  getRecorderStatus,
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
import { recorderStatusIsCurrent } from './recorder-status-order';
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
  let subscribers = 0;
  let listener: Promise<() => void> | null = null;
  async function subscribeShared() {
    subscribers += 1;
    if (listener === null) listener = subscribeToStatusEvents(set, get);
    try {
      await listener;
      let active = true;
      return () => {
        if (!active) return;
        active = false;
        subscribers -= 1;
        if (subscribers === 0 && listener !== null) {
          const current = listener;
          listener = null;
          void current.then((cleanup) => cleanup());
        }
      };
    } catch (error) {
      subscribers -= 1;
      if (subscribers === 0) listener = null;
      throw error;
    }
  }
  return {
    pause: () => pause(set, get),
    refreshRecordingStats: () => refreshRecordingStats(set, get),
    resume: () => resume(set, get),
    start: (setup) => start(set, get, setup),
    stop: () => stop(set, get),
    subscribeToStatusEvents: subscribeShared,
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
  if (get().status?.state !== 'recording' && get().status?.state !== 'paused') {
    set({ recordingStats: null, recordingStatsError: null });
    return;
  }

  try {
    const recordingStats = await getRecorderRecordingStats();
    const status = get().status;
    if (
      (status?.state !== 'recording' && status?.state !== 'paused') ||
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

async function start(
  set: RecorderWorkspaceSet,
  get: RecorderWorkspaceGet,
  setup?: SaveCaptureSetupInput,
) {
  const state = get();
  const scene = state.scene;
  if (!scene || (state.isDirty && !setup)) {
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
      ...(setup ? { setup } : {}),
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
  let receivedEvent = false;
  let active = true;
  function applyStatus(status: Awaited<ReturnType<typeof getRecorderStatus>>) {
    if (!active || !recorderStatusIsCurrent(get().status, status)) return;
    set({
      error: status.lastError?.message ?? null,
      pendingLifecycleAction: null,
      recordingStats:
        status.state === 'recording' || status.state === 'paused'
          ? get().recordingStats
          : null,
      recordingStatsError:
        status.state === 'recording' || status.state === 'paused'
          ? get().recordingStatsError
          : null,
      status,
    });
  }
  const unlisten = await listenToRecorderStatusChanged((status) => {
    receivedEvent = true;
    applyStatus(status);
  });
  // Subscribe before reading: a newer event owns state even when an older IPC
  // snapshot completes later. Re-entry must not reload the editable scene.
  const statusBeforeRead = get().status;
  const actionBeforeRead = get().pendingLifecycleAction;
  void getRecorderStatus()
    .then((snapshot) => {
      if (
        !receivedEvent &&
        get().status === statusBeforeRead &&
        get().pendingLifecycleAction === actionBeforeRead
      )
        applyStatus(snapshot);
    })
    .catch((error) => {
      if (active && !receivedEvent) set({ error: errorMessage(error) });
    });
  return () => {
    active = false;
    unlisten();
  };
}
