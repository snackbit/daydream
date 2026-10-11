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
  windowProtection: RecordWindowProtectionSettings;
}

export type ProtectedWindowAction = 'pause' | 'redact';

export type ProtectedWindowRuleAction = 'inherit' | ProtectedWindowAction;

export type ProtectedWindowTitleMatcherKind = 'contains' | 'regex';

export interface ProtectedWindowTitleMatcher {
  caseSensitive: boolean;
  kind: ProtectedWindowTitleMatcherKind;
  value: string;
}

export interface ProtectedWindowRule {
  action: ProtectedWindowRuleAction;
  appIdentity?: string | null;
  enabled: boolean;
  id: string;
  label: string;
  processName?: string | null;
  titleMatcher?: ProtectedWindowTitleMatcher | null;
}

export interface RecordWindowProtectionSettings {
  defaultAction: ProtectedWindowAction;
  dwellMs: number;
  enabled: boolean;
  mergeGapMs: number;
  rules: ProtectedWindowRule[];
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
  windowProtectionDwellMsMax: number;
  windowProtectionDwellMsMin: number;
  windowProtectionMergeGapMsMax: number;
  windowProtectionMergeGapMsMin: number;
  windowProtectionRuleCountMax: number;
  windowProtectionTextMaxChars: number;
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
  credentials?: SettingsCredentialDrafts;
  /** False for ordinary autosaves; true only after reviewing a storage-policy change. */
  applyStorageCleanup?: boolean;
}

export interface SettingsCredentialDrafts {
  modelKeys?: Record<string, string>;
  recallEmbeddingKey?: string | null;
  recallGenerationKey?: string | null;
  transcriptionKey?: string | null;
  visionKey?: string | null;
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

export type InputActivityStatusReason =
  | 'available'
  | 'dev-input-missing'
  | 'no-supported-devices'
  | 'permission-denied'
  | 'unknown'
  | 'unsupported-platform';

export interface InputActivityStatus {
  available: boolean;
  candidateDeviceCount: number;
  currentUser: string | null;
  message: string;
  permissionDeniedDeviceCount: number;
  platform: string;
  readableDeviceCount: number;
  reason: InputActivityStatusReason;
  repairAvailable: boolean;
  userInInputGroup: boolean;
}

export interface InputActivityRepairResult {
  message: string;
  status: InputActivityStatus;
  success: boolean;
}
