import type {
  AudioSource,
  AudioSourceRole,
  CaptureSource,
  RecorderPreviewFrame,
  RecorderPreviewStatus,
  RecorderRecordingStats,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import type {
  DuplicateCaptureSourceCandidate,
  DuplicateCaptureSourceResolution,
} from './scene-layout';

export type AsyncStatus = 'error' | 'idle' | 'loading' | 'ready';
export type RecorderLifecycleAction = 'pause' | 'resume' | 'start' | 'stop';

export interface RecorderWorkspaceState {
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
  recordingStats: RecorderRecordingStats | null;
  recordingStatsError: string | null;
  refreshRecordingStats: () => Promise<void>;
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

export type RecorderWorkspacePatch =
  | Partial<RecorderWorkspaceState>
  | ((state: RecorderWorkspaceState) => Partial<RecorderWorkspaceState>);
export type RecorderWorkspaceSet = (partial: RecorderWorkspacePatch) => void;
export type RecorderWorkspaceGet = () => RecorderWorkspaceState;
