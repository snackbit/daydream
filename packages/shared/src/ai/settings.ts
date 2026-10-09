import type { RecallSettings } from '../recall.js';
import type {
  AiOcrEngine,
  AiProviderKind,
  AiTranscriptionBackend,
  AiVisionProviderKind,
} from './provider-kinds.js';

export interface AiSettings {
  recall: RecallSettings;
  activity: AiActivitySettings;
  ocr: AiOcrSettings;
  processing: AiProcessingSettings;
  transcription: AiTranscriptionProviderSettings;
  vision: AiVisionProviderSettings;
}

export interface AiTranscriptionProviderSettings {
  language: string;
  localBackend: AiTranscriptionBackend;
  localModelId: string;
  maxInitialTimestampSeconds: number;
  mergeGapMs: number;
  minSpeechMs: number;
  provider: AiProviderKind;
  remoteApiKeyRef: string | null;
  remoteBaseUrl: string | null;
  remoteModel: string | null;
  silenceGateEnabled: boolean;
  silenceThresholdDb: number;
  wordTimestampsEnabled: boolean;
}

export interface AiVisionProviderSettings {
  authMode?: 'api-key' | 'none';
  observationsEnabled: boolean;
  apiKeyRef: string | null;
  baseUrl: string | null;
  model: string | null;
  provider: AiVisionProviderKind;
}

export interface TestVisionProviderInput {
  vision: AiVisionProviderSettings;
  keyDraft: string | null;
  ocr: boolean;
  observations: boolean;
}
export interface VisionProviderTestResult {
  checks: Array<{
    kind: 'ocr' | 'observations';
    status: 'succeeded' | 'failed';
    errorCode: string | null;
    elapsedMs: number;
  }>;
}

export interface AiOcrSettings {
  enabled: boolean;
  engine: AiOcrEngine;
}

export interface AiProcessingSettings {
  autoStartAfterRecording: boolean;
  timing?: 'during-recording' | 'after-stop' | 'manual';
  resourceMode?: 'background' | 'fast';
  frameRecognitionFps: number;
  frameRecognitionSyncWithRecording: boolean;
  frameSimilarityEnabled: boolean;
  maxFullFrameCheckpointsPerMinute: number;
  maxParallelChunksPerJob: number;
  maxParallelJobs: number;
  maxParallelOcrJobs: number;
  maxParallelVlmJobs: number;
  periodicFullFrameIntervalMs: number;
  visualDeltaTileSizePx: number;
}

export interface AiActivitySettings {
  frameDeltas: AiActivitySourceSettings;
  keyboard: AiActivityWindowSettings;
  mouse: AiActivityWindowSettings;
  windows: AiActivitySourceSettings;
}

export interface AiActivitySourceSettings {
  enabled: boolean;
}

export interface AiActivityWindowSettings {
  enabled: boolean;
  windowMs: number;
}

export interface AiSettingsLimits {
  activity: AiActivityLimits;
  processing: AiProcessingLimits;
  transcription: AiTranscriptionLimits;
}

export interface AiTranscriptionLimits {
  maxInitialTimestampSecondsMax: number;
  maxInitialTimestampSecondsMin: number;
  mergeGapMsMax: number;
  mergeGapMsMin: number;
  minSpeechMsMax: number;
  minSpeechMsMin: number;
  silenceThresholdDbMax: number;
  silenceThresholdDbMin: number;
}

export interface AiProcessingLimits {
  frameRecognitionFpsMax: number;
  frameRecognitionFpsMin: number;
  maxFullFrameCheckpointsPerMinuteMax: number;
  maxFullFrameCheckpointsPerMinuteMin: number;
  maxParallelChunksPerJobMax: number;
  maxParallelChunksPerJobMin: number;
  maxParallelJobsMax: number;
  maxParallelJobsMin: number;
  maxParallelOcrJobsMax: number;
  maxParallelOcrJobsMin: number;
  maxParallelVlmJobsMax: number;
  maxParallelVlmJobsMin: number;
  periodicFullFrameIntervalMsMax: number;
  periodicFullFrameIntervalMsMin: number;
  visualDeltaTileSizePxMax: number;
  visualDeltaTileSizePxMin: number;
}

export interface AiActivityLimits {
  keyboardWindowMsMax: number;
  keyboardWindowMsMin: number;
  mouseWindowMsMax: number;
  mouseWindowMsMin: number;
}
