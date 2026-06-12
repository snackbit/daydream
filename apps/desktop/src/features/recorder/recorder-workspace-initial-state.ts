import type { RecorderWorkspaceState } from './recorder-workspace-store-types';

export const recorderWorkspaceInitialState = {
  audioSources: [],
  captureSources: [],
  desktopAudioSourceId: null,
  error: null,
  isDirty: false,
  loadStatus: 'idle',
  microphoneAudioSourceId: null,
  pendingDuplicateCaptureSources: [],
  pendingLifecycleAction: null,
  previewFramesBySceneSourceId: {},
  previewStartInFlightSceneId: null,
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
  | 'desktopAudioSourceId'
  | 'error'
  | 'isDirty'
  | 'loadStatus'
  | 'microphoneAudioSourceId'
  | 'pendingDuplicateCaptureSources'
  | 'pendingLifecycleAction'
  | 'previewFramesBySceneSourceId'
  | 'previewStartInFlightSceneId'
  | 'previewStatus'
  | 'recordingStats'
  | 'recordingStatsError'
  | 'saveStatus'
  | 'scene'
  | 'selectedSceneSourceId'
  | 'status'
>;
