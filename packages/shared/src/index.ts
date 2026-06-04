export interface AppDiagnostics {
  identifier: string;
  name: string;
  version: string;
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
  ffmpeg: FfmpegDiagnostics;
  storage: StorageDiagnostics;
  system: SystemDiagnostics;
}
