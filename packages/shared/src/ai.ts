export type AiProviderKind = 'local-whisper-cpp' | 'openai-compatible';

export type AiVisionProviderKind = 'disabled' | 'openai-compatible';

export type AiOcrEngine = 'tesseract';

export type AiModelDownloadStatus =
  | 'available'
  | 'downloading'
  | 'failed'
  | 'missing';

export type AiModelSelfTestStatus = 'failed' | 'not-run' | 'passed' | 'running';

export type ProcessingJobKind =
  | 'activity'
  | 'ai-processing'
  | 'chunking'
  | 'digest'
  | 'embedding'
  | 'frame-extraction'
  | 'ocr'
  | 'transcription'
  | 'vlm';

export type ProcessingJobStatus =
  | 'cancelled'
  | 'failed'
  | 'paused'
  | 'queued'
  | 'running'
  | 'succeeded';

export type ProcessingChunkSourceKind =
  | 'activity'
  | 'audio'
  | 'frame'
  | 'ocr'
  | 'vlm';

export type ProcessingChunkStatus =
  | 'cancelled'
  | 'failed'
  | 'queued'
  | 'running'
  | 'succeeded';

export type AiNotificationKind =
  | 'model-download'
  | 'model-self-test'
  | 'processing-job'
  | 'provider-status';

export type AiNotificationStatus =
  | 'cancelled'
  | 'failed'
  | 'info'
  | 'running'
  | 'succeeded'
  | 'warning';

export type TranscriptSourceRole = 'desktop-audio' | 'microphone';

export type ActivitySourceKind =
  | 'frame-delta'
  | 'keyboard'
  | 'merged'
  | 'mouse';

export type ActivityIntervalState = 'active' | 'idle' | 'unknown';

export interface AiSettings {
  activity: AiActivitySettings;
  ocr: AiOcrSettings;
  processing: AiProcessingSettings;
  transcription: AiTranscriptionProviderSettings;
  vision: AiVisionProviderSettings;
}

export interface AiTranscriptionProviderSettings {
  localModelId: string;
  provider: AiProviderKind;
  remoteApiKeyRef: string | null;
  remoteBaseUrl: string | null;
  wordTimestampsEnabled: boolean;
}

export interface AiVisionProviderSettings {
  apiKeyRef: string | null;
  baseUrl: string | null;
  model: string | null;
  provider: AiVisionProviderKind;
}

export interface AiOcrSettings {
  enabled: boolean;
  engine: AiOcrEngine;
}

export interface AiProcessingSettings {
  autoStartAfterRecording: boolean;
  frameSimilarityEnabled: boolean;
  maxParallelChunksPerJob: number;
  maxParallelJobs: number;
  periodicFullFrameIntervalMs: number;
}

export interface AiActivitySettings {
  frameDeltas: AiActivitySourceSettings;
  keyboard: AiActivityWindowSettings;
  mouse: AiActivityWindowSettings;
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
}

export interface AiProcessingLimits {
  maxParallelChunksPerJobMax: number;
  maxParallelChunksPerJobMin: number;
  maxParallelJobsMax: number;
  maxParallelJobsMin: number;
  periodicFullFrameIntervalMsMax: number;
  periodicFullFrameIntervalMsMin: number;
}

export interface AiActivityLimits {
  keyboardWindowMsMax: number;
  keyboardWindowMsMin: number;
  mouseWindowMsMax: number;
  mouseWindowMsMin: number;
}

export interface AiModelAsset {
  createdAtMs: number;
  downloadStatus: AiModelDownloadStatus;
  etag: string | null;
  id: string;
  label: string;
  localPath: string | null;
  provider: AiProviderKind;
  selfTestStatus: AiModelSelfTestStatus;
  sha256: string | null;
  sizeBytes: number | null;
  updatedAtMs: number;
}

export interface AiModelAssetList {
  models: AiModelAsset[];
}

export interface ProcessingJob {
  createdAtMs: number;
  errorCode: string | null;
  id: string;
  kind: ProcessingJobKind;
  progress: number;
  recordingId: string;
  status: ProcessingJobStatus;
  updatedAtMs: number;
}

export interface ProcessingChunk {
  attemptCount: number;
  createdAtMs: number;
  endOffsetMs: number | null;
  errorCode: string | null;
  id: string;
  jobId: string;
  progress: number;
  recordingId: string;
  segmentId: string | null;
  sourceKind: ProcessingChunkSourceKind;
  startOffsetMs: number | null;
  status: ProcessingChunkStatus;
  updatedAtMs: number;
}

export interface ProcessingJobList {
  jobs: ProcessingJob[];
}

export interface ListProcessingJobsInput {
  recordingId?: string | null;
}

export interface ProcessingJobActionInput {
  jobId: string;
}

export interface AiNotification {
  createdAtMs: number;
  dismissedAtMs: number | null;
  id: string;
  jobId: string | null;
  kind: AiNotificationKind;
  message: string;
  status: AiNotificationStatus;
  title: string;
  updatedAtMs: number;
}

export interface AiNotificationList {
  notifications: AiNotification[];
}

export interface DismissAiNotificationInput {
  notificationId: string;
}

export interface RecordingAiResultInput {
  recordingId: string;
}

export interface TranscriptSegment {
  chunkId: string;
  confidence: number | null;
  endOffsetMs: number;
  id: string;
  jobId: string;
  model: string | null;
  provider: AiProviderKind;
  recordingId: string;
  segmentId: string;
  sourceRole: TranscriptSourceRole;
  startOffsetMs: number;
  text: string;
  trackId: string;
  words: TranscriptWord[];
}

export interface TranscriptWord {
  endOffsetMs: number;
  id: string;
  probability: number | null;
  startOffsetMs: number;
  text: string;
  transcriptSegmentId: string;
}

export interface RecordingTranscript {
  recordingId: string;
  segments: TranscriptSegment[];
}

export interface ActivityInterval {
  endOffsetMs: number;
  id: string;
  jobId: string;
  recordingId: string;
  score: number | null;
  sourceKind: ActivitySourceKind;
  startOffsetMs: number;
  state: ActivityIntervalState;
}

export interface RecordingActivity {
  intervals: ActivityInterval[];
  recordingId: string;
}

export interface VisualRegion {
  height: number;
  width: number;
  x: number;
  y: number;
}

export interface OcrRegion {
  chunkId: string;
  confidence: number | null;
  frameTimestampMs: number;
  id: string;
  jobId: string;
  model: string | null;
  provider: string;
  recordingId: string;
  region: VisualRegion;
  sourceFrameRef: string;
  text: string;
}

export interface VlmObservation {
  chunkId: string;
  confidence: number | null;
  frameTimestampMs: number;
  id: string;
  jobId: string;
  model: string | null;
  promptVersion: string;
  provider: AiVisionProviderKind;
  recordingId: string;
  region: VisualRegion;
  sourceFrameRef: string;
  text: string;
}

export interface RecordingVisualObservations {
  ocrRegions: OcrRegion[];
  recordingId: string;
  vlmObservations: VlmObservation[];
}
