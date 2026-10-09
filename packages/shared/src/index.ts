import type {
  AiProviderKind,
  AiTranscriptionBackend,
  ProcessingChunkSourceKind,
  ProcessingChunkStatus,
  ProcessingJobStatus,
} from './ai.js';
import type {
  CaptureTimingDiagnostics,
  RecorderBackendHealth,
  RecorderMediaWriterCapabilities,
  RecorderPreviewStatus,
  RecorderSegmentState,
} from './recorder.js';

export interface AppDiagnostics {
  identifier: string;
  name: string;
  version: string;
}

export interface DatabaseDiagnostics {
  available: boolean;
  error: string | null;
  path: string;
  schemaVersion: number | null;
  tableCounts: Record<string, number>;
}

export interface MediaWriterDiagnostics {
  available: boolean;
  capabilities: RecorderMediaWriterCapabilities;
  error: string | null;
  version: string | null;
  writer: string | null;
}

export interface AiProcessingDiagnostics {
  chunkCount: number;
  chunkStatusCounts: Record<string, number>;
  chunks: AiProcessingChunkDiagnostics[];
  error: string | null;
  latestJob: AiProcessingJobDiagnostics | null;
  recordingId: string | null;
  settings: AiProcessingSettingsDiagnostics | null;
  transcriptTracks: AiTranscriptTrackDiagnostics[];
}

export interface AiProcessingSettingsDiagnostics {
  autoStartAfterRecording: boolean;
  maxParallelChunksPerJob: number;
  maxParallelJobs: number;
  maxParallelOcrJobs: number;
  maxParallelVlmJobs: number;
  transcriptionBackend: AiTranscriptionBackend;
  transcriptionLanguage: string;
  transcriptionMaxInitialTimestampSeconds: number;
  transcriptionMergeGapMs: number;
  transcriptionMinSpeechMs: number;
  transcriptionModel: string;
  transcriptionProvider: AiProviderKind;
  transcriptionSilenceGateEnabled: boolean;
  transcriptionSilenceThresholdDb: number;
  wordTimestampsEnabled: boolean;
}

export interface AiProcessingJobDiagnostics {
  createdAtMs: number;
  error: string | null;
  id: string;
  kind: string;
  status: ProcessingJobStatus;
  updatedAtMs: number;
}

export interface AiProcessingChunkDiagnostics {
  attemptCount: number;
  endOffsetMs: number | null;
  errorCode: string | null;
  id: string;
  progress: number;
  resultCode: string | null;
  segmentId: string | null;
  sourceKind: ProcessingChunkSourceKind;
  startOffsetMs: number | null;
  status: ProcessingChunkStatus;
  trackId: string | null;
  trackKind: string | null;
  trackLabel: string | null;
  updatedAtMs: number;
}

export interface AiTranscriptTrackDiagnostics {
  firstStartOffsetMs: number | null;
  lastEndOffsetMs: number | null;
  segmentCount: number;
  sourceRole: string;
  textCharCount: number;
  trackId: string;
}

export interface OcrDiagnostics {
  dataDownloadStatus: string;
  dataPathPresent: boolean;
  dataSource: string;
  enabled: boolean;
  engine: string;
  languages: string[];
  runtimeDownloadStatus: string;
  runtimePath: string | null;
  runtimePathPresent: boolean;
  runtimeSource: string;
  runtimeVersion: string | null;
}

export type SegmentProbeMetadata = Record<string, unknown>;

export interface LatestSegmentDiagnostics {
  desktopAudioStreamIndex: number | null;
  durationMs: number | null;
  endedAtMs: number | null;
  errorCode: string | null;
  writerExitCode: number | null;
  filePath: string;
  id: string;
  microphoneAudioStreamIndex: number | null;
  probe: SegmentProbeMetadata | null;
  recordingId: string;
  segmentIndex: number;
  startedAtMs: number;
  state: RecorderSegmentState;
  videoStreamIndex: number | null;
}

export interface LatestRecordingDiagnostics {
  activeCaptureTiming: CaptureTimingDiagnostics | null;
  activeRecordingId: string | null;
  activeSegmentId: string | null;
  error: string | null;
  latestSegment: LatestSegmentDiagnostics | null;
  previewStatus: RecorderPreviewStatus | null;
  recordingId: string | null;
  segmentCount: number;
}

export interface StorageDiagnostics {
  appDataDir: string | null;
}

export interface SystemDiagnostics {
  arch: string;
  family: string;
  platform: string;
}

export interface DaydreamDiagnostics {
  aiProcessing: AiProcessingDiagnostics;
  app: AppDiagnostics;
  database: DatabaseDiagnostics;
  debugLogging: DebugLoggingDiagnostics;
  latestRecording: LatestRecordingDiagnostics;
  mediaWriter: MediaWriterDiagnostics;
  ocr: OcrDiagnostics;
  portalRestoreToken: PortalRestoreTokenDiagnostics;
  recorder: RecorderBackendHealth;
  storage: StorageDiagnostics;
  system: SystemDiagnostics;
}

export * from './recorder.js';
export * from './library.js';
export * from './ai.js';
export * from './settings.js';
export * from './startup.js';
export * from './window-protection.js';
export * from './recall.js';
export * from './maintenance.js';
export * from './recovery.js';

export interface DebugLoggingDiagnostics {
  enabled: boolean;
  filePath: string | null;
  filter: string;
  format: string;
  latestFileBytes: number | null;
  retentionDays: number;
  stdout: boolean;
}

export interface PortalRestoreTokenDiagnostics {
  createdAtMs: number | null;
  error: string | null;
  expectedId: string;
  lastUsedAtMs: number | null;
  portal: string | null;
  present: boolean;
  tokenLength: number | null;
  updatedAtMs: number | null;
}

export interface DebugLogBundle {
  diagnostics: DaydreamDiagnostics;
  logTailLines: string[];
}
