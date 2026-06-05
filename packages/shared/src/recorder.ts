export type RecorderState =
  | 'error'
  | 'idle'
  | 'paused'
  | 'recording'
  | 'selecting'
  | 'stopping'
  | 'unavailable';

export type RecorderErrorCategory =
  | 'audio'
  | 'dependency'
  | 'encoder'
  | 'filesystem'
  | 'lifecycle'
  | 'permission'
  | 'unavailable'
  | 'video';

export interface RecorderError {
  category: RecorderErrorCategory;
  code: string;
  details?: Record<string, boolean | null | number | string>;
  message: string;
}

export interface CanvasRect {
  height: number;
  width: number;
  x: number;
  y: number;
}

export interface CropRect {
  bottom: number;
  left: number;
  right: number;
  top: number;
}

export type CaptureSourceKind = 'monitor';

export interface CaptureSource {
  displayName: string;
  height: number | null;
  id: string;
  kind: CaptureSourceKind;
  logicalHeight: number | null;
  logicalWidth: number | null;
  scaleFactor: number | null;
  width: number | null;
}

export interface CaptureSourceList {
  sources: CaptureSource[];
}

export interface CaptureSourceSelection {
  sourceIds: string[];
}

export interface SelectCaptureSourcesInput {
  forceChooser: boolean;
}

export type AudioSourceRole = 'desktop-audio' | 'microphone';

export type AudioSourceKind = 'input' | 'monitor';

export interface AudioSource {
  displayName: string;
  id: string;
  isDefault: boolean;
  kind: AudioSourceKind;
  roles: AudioSourceRole[];
}

export interface AudioSourceList {
  sources: AudioSource[];
}

export type RecorderSceneLayoutMode = 'custom' | 'horizontal';

export interface RecorderSceneSource {
  crop: CropRect;
  displayName: string;
  id: string;
  preserveAspectRatio: boolean;
  rect: CanvasRect;
  sourceId: string;
  sourceKind: CaptureSourceKind;
  zIndex: number;
}

export interface RecorderScene {
  canvasHeight: number;
  canvasWidth: number;
  id: string;
  layoutMode: RecorderSceneLayoutMode;
  name: string;
  sources: RecorderSceneSource[];
}

export interface SaveRecorderSceneInput {
  scene: RecorderScene;
}

export type RecorderQualityPreset = 'balanced-ocr';

export interface StartRecordingInput {
  desktopAudioSourceId: string | null;
  fps: number;
  microphoneAudioSourceId: string | null;
  qualityPreset: RecorderQualityPreset;
  sceneId: string;
}

export interface PauseRecordingInput {
  recordingId: string;
}

export interface ResumeRecordingInput {
  recordingId: string;
}

export interface StopRecordingInput {
  recordingId: string;
}

export type RecorderSegmentState = 'failed' | 'finalized' | 'recording';

export interface RecorderSegment {
  desktopAudioStreamIndex: number | null;
  durationMs: number | null;
  endedAtMs: number | null;
  errorCode: string | null;
  ffmpegExitCode: number | null;
  filePath: string;
  id: string;
  microphoneAudioStreamIndex: number | null;
  recordingId: string;
  segmentIndex: number;
  startedAtMs: number;
  state: RecorderSegmentState;
  videoStreamIndex: number | null;
}

export interface RecorderBackendComponentHealth {
  available: boolean;
  error: string | null;
}

export interface RecorderFfmpegHealth extends RecorderBackendComponentHealth {
  path: string | null;
}

export interface RecorderAudioHealth extends RecorderBackendComponentHealth {
  defaultSourceId: string | null;
}

export interface RecorderPortalHealth extends RecorderBackendComponentHealth {
  cursorModes: string[];
  screenCastVersion: number | null;
  sourceTypes: string[];
}

export interface RecorderBackendHealth {
  desktopAudio: RecorderAudioHealth;
  ffmpeg: RecorderFfmpegHealth;
  microphone: RecorderAudioHealth;
  pipewire: RecorderBackendComponentHealth;
  portal: RecorderPortalHealth;
}

export interface RecorderStatus {
  activeRecordingId: string | null;
  activeSceneId: string | null;
  activeSegmentId: string | null;
  backendHealth: RecorderBackendHealth;
  lastError: RecorderError | null;
  state: RecorderState;
  updatedAtMs: number;
}
