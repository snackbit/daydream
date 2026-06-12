import type {
  AiProviderKind,
  AiTranscriptionBackend,
} from './provider-kinds.js';

export type AiModelDownloadStatus =
  | 'available'
  | 'cancelled'
  | 'downloading'
  | 'failed'
  | 'missing'
  | 'queued';

export type AiModelSelfTestStatus = 'failed' | 'not-run' | 'passed' | 'running';

export type AiModelVerificationStatus =
  | 'failed'
  | 'passed'
  | 'pending'
  | 'unknown';

export interface AiModelAsset {
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
  provider: AiProviderKind;
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

export interface AiProviderStatus {
  localTranscription: AiLocalTranscriptionStatus;
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
