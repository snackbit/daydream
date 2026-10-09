import type {
  AiAssetProviderKind,
  AiTranscriptionBackend,
} from './provider-kinds.js';

export type AiModelDownloadStatus =
  | 'available'
  | 'cancelled'
  | 'downloading'
  | 'failed'
  | 'missing'
  | 'paused'
  | 'queued';

export interface AiModelTransfer {
  attempt: number;
  phase:
    | 'queued'
    | 'connecting'
    | 'restarting'
    | 'transferring'
    | 'stalled'
    | 'pausing'
    | 'paused'
    | 'verifying'
    | 'testing'
    | 'ready'
    | 'failed'
    | 'cancelled';
  bytesPerSecond: number | null;
  etaMs: number | null;
}

export type AiOcrDataStatus = AiModelDownloadStatus | 'partial';

export type AiOcrDataSource = 'managed-best' | 'missing' | 'partial' | 'system';

export type AiOcrRuntimeSource = 'managed' | 'missing' | 'system';

export type AiModelSelfTestStatus = 'failed' | 'not-run' | 'passed' | 'running';

export type AiModelVerificationStatus =
  'failed' | 'passed' | 'pending' | 'unknown';

export interface AiModelAsset {
  transfer?: AiModelTransfer;
  checksum: string | null;
  checksumAlgorithm: string | null;
  createdAtMs: number;
  downloadedBytes: number;
  downloadStatus: AiModelDownloadStatus;
  etag: string | null;
  errorCode: string | null;
  id: string;
  label: string;
  localPath: string | null;
  provider: AiAssetProviderKind;
  selfTestStatus: AiModelSelfTestStatus;
  sizeBytes: number | null;
  sourceUrl: string | null;
  totalBytes: number | null;
  updatedAtMs: number;
  verificationStatus: AiModelVerificationStatus;
}

export interface AiModelAssetList {
  models: AiModelAsset[];
}

export interface AiBackendStatus {
  available: boolean;
  id: AiTranscriptionBackend;
  reason: string | null;
}

export interface AiTranscriptionLanguageStatus {
  id: string;
  label: string;
}

export interface AiLocalTranscriptionStatus {
  availableLanguages: AiTranscriptionLanguageStatus[];
  availableBackends: AiBackendStatus[];
  checksumStatus: AiModelVerificationStatus;
  localModelId: string;
  modelDownloadStatus: AiModelDownloadStatus;
  modelPath: string | null;
  selectedBackend: AiTranscriptionBackend;
  selfTestStatus: AiModelSelfTestStatus;
}

export interface AiOcrStatus {
  dataSource: AiOcrDataSource;
  dataDownloadStatus: AiOcrDataStatus;
  dataPath: string | null;
  enabled: boolean;
  engine: string;
  languages: string[];
  runtimeDownloadStatus: AiModelDownloadStatus;
  runtimePath: string | null;
  runtimeSource: AiOcrRuntimeSource;
  runtimeVersion: string | null;
}

export interface AiProviderStatus {
  localTranscription: AiLocalTranscriptionStatus;
  ocr: AiOcrStatus;
  remoteTranscription: string;
  vision: string;
}

export interface DownloadAiModelInput {
  modelId: string;
  notificationId?: string;
}

export interface CancelAiModelDownloadInput {
  modelId: string;
  notificationId?: string;
}

export interface DeleteAiModelInput {
  modelId: string;
}

export interface SelfTestAiModelInput {
  modelId: string;
}

export interface SaveAiProviderSecretInput {
  plaintext: string;
  role: 'transcription' | 'vision';
}

export interface DeleteAiProviderSecretInput {
  secretRef: string;
}

export interface SaveAiProviderSecretResult {
  secretRef: string;
}
