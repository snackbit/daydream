import type { RecorderWorkspaceState } from './recorder-workspace-store-types';

export const recorderWorkspaceInitialState = {
  audioSources: [],
  captureSources: [],
  captureSelectionPending: false,
  desktopAudioSourceId: null,
  error: null,
  isDirty: false,
  workspaceRevision: 0,
  loadStatus: 'idle',
  microphoneAudioSourceId: null,
  pendingDuplicateCaptureSources: [],
  pendingLifecycleAction: null,
  previewFramesBySceneSourceId: {},
  previewStartInFlightSceneId: null,
  previewOperationGeneration: 0,
  previewStopInFlight: false,
  previewStatus: null,
  recordingStats: null,
  recordingStatsError: null,
  saveStatus: 'idle',
  scene: null,
  selectedSceneSourceId: null,
  status: null,
} satisfies Pick<
  RecorderWorkspaceState,
  | 'audioSources'
  | 'captureSources'
  | 'captureSelectionPending'
  | 'desktopAudioSourceId'
  | 'error'
  | 'isDirty'
  | 'workspaceRevision'
  | 'loadStatus'
  | 'microphoneAudioSourceId'
  | 'pendingDuplicateCaptureSources'
  | 'pendingLifecycleAction'
  | 'previewFramesBySceneSourceId'
  | 'previewStartInFlightSceneId'
  | 'previewOperationGeneration'
  | 'previewStopInFlight'
  | 'previewStatus'
  | 'recordingStats'
  | 'recordingStatsError'
  | 'saveStatus'
  | 'scene'
  | 'selectedSceneSourceId'
  | 'status'
>;
