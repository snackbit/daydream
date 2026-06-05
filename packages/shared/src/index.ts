import type {
  CaptureTimingDiagnostics,
  RecorderBackendHealth,
  RecorderFfmpegCapabilities,
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

export interface FfmpegDiagnostics {
  available: boolean;
  capabilities: RecorderFfmpegCapabilities;
  error: string | null;
  path: string | null;
  source: 'daydream-ffmpeg-env' | 'path' | 'unconfigured';
  versionLine: string | null;
}

export type SegmentProbeMetadata = Record<string, unknown>;

export interface LatestSegmentDiagnostics {
  desktopAudioStreamIndex: number | null;
  durationMs: number | null;
  endedAtMs: number | null;
  errorCode: string | null;
  ffmpegExitCode: number | null;
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
  ffmpeg: FfmpegDiagnostics;
  latestRecording: LatestRecordingDiagnostics;
  recorder: RecorderBackendHealth;
  storage: StorageDiagnostics;
  system: SystemDiagnostics;
}

export * from './recorder.js';
