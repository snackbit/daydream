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
  app: AppDiagnostics;
  database: DatabaseDiagnostics;
  debugLogging: DebugLoggingDiagnostics;
  latestRecording: LatestRecordingDiagnostics;
  mediaWriter: MediaWriterDiagnostics;
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
