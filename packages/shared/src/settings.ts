import type { AiSettings, AiSettingsLimits } from './ai.js';

export interface AppSettingsState {
  defaults: AppSettings;
  limits: AppSettingsLimits;
  settings: AppSettings;
  storageCleanup?: AppSettingsStorageCleanupResult | null;
}

export interface AppSettings {
  ai: AiSettings;
  debug?: DebugSettings;
  expert?: ExpertSettings;
  record: RecordSettings;
  storage: StorageSettings;
}

export interface DebugSettings {
  enabled: boolean;
}

export interface ExpertSettings {
  enabled: boolean;
}

export interface RecordSettings {
  fps: number;
  videoBitrateKbps: number;
}

export interface StorageSettings {
  retentionDays: number;
  sizeLimitBytes: number;
}

export interface AppSettingsLimits {
  ai: AiSettingsLimits;
  record: RecordSettingsLimits;
  storage: StorageSettingsLimits;
}

export interface RecordSettingsLimits {
  fpsMax: number;
  fpsMin: number;
  videoBitrateKbpsMax: number;
  videoBitrateKbpsMin: number;
}

export interface StorageSettingsLimits {
  retentionDaysMax: number;
  retentionDaysMin: number;
  sizeLimitBytesMax: number | null;
  sizeLimitBytesMin: number;
}

export interface SaveAppSettingsInput {
  settings: AppSettings;
}

export interface PreviewAppSettingsStorageCleanupInput {
  settings: AppSettings;
}

export interface AppSettingsStorageCleanupPreview {
  recordingCount: number;
  sizeBytes: number;
}

export interface AppSettingsStorageCleanupResult {
  deletedRecordingCount: number;
  deletedSizeBytes: number;
  failedRecordingCount: number;
}

export interface AppSettingsError {
  code: string;
  field?: string;
  message: string;
}
