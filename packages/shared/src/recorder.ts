export type RecorderState =
  | 'error'
  | 'idle'
  | 'pausing'
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

export type RecorderSceneLayoutMode =
  'auto-16-9' | 'custom' | 'horizontal' | 'vertical';

export interface RecorderSceneSource {
  crop: CropRect;
  displayName: string;
  id: string;
  preserveAspectRatio: boolean;
  rect: CanvasRect;
  sourceId: string;
  sourceHeight: number | null;
  sourceKind: CaptureSourceKind;
  sourceWidth: number | null;
  zIndex: number;
}

export interface RecorderScene {
  canvasHeight: number;
  canvasWidth: number;
  desktopAudioSourceId: string | null;
  id: string;
  layoutMode: RecorderSceneLayoutMode;
  microphoneAudioSourceId: string | null;
  name: string;
  sources: RecorderSceneSource[];
}

export interface SaveRecorderSceneInput {
  scene: RecorderScene;
}

export interface StartRecorderPreviewInput {
  sceneId: string;
}

export type RecorderPreviewState = 'error' | 'idle' | 'live' | 'starting';

export interface RecorderPreviewStatus {
  error: RecorderError | null;
  sceneId: string | null;
  sourceIds: string[];
  state: RecorderPreviewState;
  updatedAtMs: number;
}

export interface RecorderPreviewFrame {
  dataUrl: string;
  height: number;
  sceneSourceId: string;
  sourceId: string;
  timestampMs: number;
  width: number;
}

export type RecorderQualityPreset = 'balanced-ocr';

export interface StartRecordingInput {
  desktopAudioSourceId: string | null;
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

export type RecorderFinalizationAction = 'pause' | 'stop';

export type RecorderFinalizationPhase = 'saving-segment' | 'stopping-capture';

export interface RecorderFinalizationStatus {
  action: RecorderFinalizationAction;
  phase: RecorderFinalizationPhase;
  recordingId: string;
  segmentId: string | null;
  startedAtMs: number;
}

export interface RecorderSegment {
  desktopAudioStreamIndex: number | null;
  durationMs: number | null;
  endedAtMs: number | null;
  errorCode: string | null;
  writerExitCode: number | null;
  filePath: string;
  id: string;
  microphoneAudioStreamIndex: number | null;
  recordingId: string;
  segmentIndex: number;
  startedAtMs: number;
  state: RecorderSegmentState;
  videoStreamIndex: number | null;
}

export type CaptureFrameStorageKind = 'cpu-readable' | 'dma-buf';

export interface CaptureTimingDiagnostics {
  composedFrames: number;
  cpuReadableFramesReceived: number;
  dmabufFramesReadBackToCpu: number;
  dmabufFramesReceived: number;
  framesReceivedBySource: Record<string, number>;
  framesSentToWriter: number;
  lastComposedTimestampMs: number | null;
  latestFrameStorageBySource: Record<string, CaptureFrameStorageKind>;
  maxComposeDurationMs: number;
  reusedFrameCompositions: number;
  schedulerMissedDeadlines: number;
  schedulerTicks: number;
  sourceTimestampMaxMs: number | null;
  sourceTimestampMinMs: number | null;
  totalComposeDurationMs: number;
}

export interface RecorderBackendComponentHealth {
  available: boolean;
  error: string | null;
}

export interface RecorderMediaWriterHealth extends RecorderBackendComponentHealth {
  capabilities: RecorderMediaWriterCapabilities;
  version: string | null;
  writer: string | null;
}

export interface RecorderMediaWriterCapabilities {
  appsrc: boolean;
  audioConvert: boolean;
  audioResample: boolean;
  fileSink: boolean;
  h264Parse: boolean;
  matroskaMuxer: boolean;
  opusEncoder: boolean;
  pulseSource: boolean;
  queue: boolean;
  videoConvert: boolean;
  x264Encoder: boolean;
}

export interface RecorderAudioHealth extends RecorderBackendComponentHealth {
  defaultSourceId: string | null;
}

export interface RecorderPortalHealth extends RecorderBackendComponentHealth {
  cursorModes: string[];
  screenCastVersion: number | null;
  sourceTypes: string[];
}

export interface RecorderGpuCaptureHealth extends RecorderBackendComponentHealth {
  cleanupError: string | null;
  dmabufImportAvailable: boolean;
  eglAvailable: boolean;
  eglClientApis: string[];
  eglPlatform: string | null;
  eglVendor: string | null;
  eglVersion: string | null;
  modifierImportAvailable: boolean | null;
  openglAvailable: boolean;
  renderer: string | null;
  vendor: string | null;
  version: string | null;
}

export interface RecorderBackendHealth {
  desktopAudio: RecorderAudioHealth;
  gpuCapture: RecorderGpuCaptureHealth;
  mediaWriter: RecorderMediaWriterHealth;
  microphone: RecorderAudioHealth;
  pipewire: RecorderBackendComponentHealth;
  portal: RecorderPortalHealth;
}

export interface RecorderStatus {
  activeRecordingId: string | null;
  activeSceneId: string | null;
  activeSegmentId: string | null;
  backendHealth: RecorderBackendHealth;
  finalization: RecorderFinalizationStatus | null;
  lastError: RecorderError | null;
  state: RecorderState;
  updatedAtMs: number;
}

export interface RecorderPerformanceStats {
  desiredFps: number | null;
  segmentStartedAtMs: number | null;
  state: RecorderState;
  timing: CaptureTimingDiagnostics | null;
  updatedAtMs: number;
}

export interface RecorderRecordingStats {
  activeSegmentId: string | null;
  activeSegmentIndex: number | null;
  activeSegmentSizeBytes: number | null;
  activeSegmentStartedAtMs: number | null;
  recordingId: string;
  recordingStartedAtMs: number | null;
  segmentCount: number;
  totalSizeBytes: number;
  updatedAtMs: number;
}
