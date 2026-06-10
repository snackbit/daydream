export type AiProviderKind = 'local-whisper-cpp' | 'openai-compatible';

export type AiVisionProviderKind = 'disabled' | 'openai-compatible';

export type AiOcrEngine = 'tesseract';

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

export type AiTranscriptionBackend = 'cpu' | 'cuda' | 'vulkan';

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

export type NotificationSource =
  | 'ai'
  | 'capture'
  | 'library'
  | 'recorder'
  | 'settings'
  | 'system';

export type NotificationStatus =
  | 'cancelled'
  | 'failed'
  | 'info'
  | 'running'
  | 'succeeded'
  | 'warning';

export type NotificationActionStyle = 'default' | 'destructive' | 'secondary';

export type NotificationActionHandler =
  | 'model-download-cancel'
  | 'model-download-retry'
  | 'processing-job-cancel'
  | 'processing-job-pause'
  | 'processing-job-resume'
  | 'processing-job-retry';

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
  localBackend: AiTranscriptionBackend;
  localModelId: string;
  provider: AiProviderKind;
  remoteApiKeyRef: string | null;
  remoteBaseUrl: string | null;
  remoteModel: string | null;
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

export interface AiLocalTranscriptionStatus {
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

export interface StartRecordingAiProcessingInput {
  recordingId: string;
}

export interface GetRecordingTranscriptInput {
  recordingId: string;
}

export interface RecordingTranscript {
  recordingId: string;
  tracks: RecordingTranscriptTrack[];
}

export interface RecordingTranscriptTrack {
  sourceRole: TranscriptSourceRole;
  trackId: string;
  segments: RecordingTranscriptSegment[];
}

export interface RecordingTranscriptSegment {
  confidence: number | null;
  endOffsetMs: number;
  id: string;
  model: string | null;
  provider: string;
  segmentId: string;
  startOffsetMs: number;
  text: string;
  words: RecordingTranscriptWord[];
}

export interface RecordingTranscriptWord {
  endOffsetMs: number;
  id: string;
  probability: number | null;
  startOffsetMs: number;
  text: string;
}

export interface NotificationAction {
  handler: NotificationActionHandler;
  id: string;
  label: string;
  payload: Record<string, string>;
  style: NotificationActionStyle;
}

export interface Notification {
  actions: NotificationAction[];
  createdAtMs: number;
  dismissedAtMs: number | null;
  id: string;
  message: string;
  source: NotificationSource;
  status: NotificationStatus;
  title: string;
  topic: string;
  updatedAtMs: number;
}

export interface NotificationList {
  notifications: Notification[];
}

export interface AcknowledgeNotificationsInput {
  notificationIds: string[];
}

export interface DismissNotificationInput {
  notificationId: string;
}

export interface DeleteNotificationsInput {
  notificationIds: string[];
}

export interface ExecuteNotificationActionInput {
  actionId: string;
  notificationId: string;
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
