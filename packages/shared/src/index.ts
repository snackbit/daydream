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
  error: string | null;
  path: string | null;
  source: 'daydream-ffmpeg-env' | 'unconfigured';
  versionLine: string | null;
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
  storage: StorageDiagnostics;
  system: SystemDiagnostics;
}

export * from './recorder.js';
