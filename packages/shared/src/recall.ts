/** Recall reads always revalidate source eligibility before returning private content. */
export type RecallSourceKind = 'ocr' | 'desktop-audio' | 'microphone';
export type RecallJobStatus =
  'pending' | 'running' | 'paused' | 'succeeded' | 'failed' | 'cancelled';
export interface RecallCitation {
  evidenceId: string;
  recordingId: string;
  segmentId: string;
  sourceKind: RecallSourceKind;
  sourceTrackId: string;
  sourceId: string;
  sourceFrameRef: string | null;
  documentId: string | null;
  startOffsetMs: number;
  endOffsetMs: number;
  absoluteStartMs: number;
  absoluteEndMs: number;
  contentRevision: string;
}
export interface RecallEvidence extends RecallCitation {
  text: string;
}
export interface RecallSearchHit {
  evidence: RecallEvidence;
  score: number;
}
export interface RecallSearchInput {
  query: string;
  dayId: string | null;
  limit: number;
  timezone?: string;
  sourceKinds?: RecallSourceKind[];
  fromMs?: number | null;
  toMs?: number | null;
  cursor?: string | null;
  mode?: 'hybrid' | 'lexical';
}
export interface RecallSearchResult {
  privacyRevision: number;
  hits: RecallSearchHit[];
  complete: boolean;
  unavailableReason: string | null;
  nextCursor: string | null;
  semanticUsed: boolean;
  indexedEvidenceCount: number;
  eligibleEvidenceCount: number;
}
export interface RecallDayCard {
  privacyRevision: number;
  dayId: string;
  startMs: number;
  endMs: number;
  timezone: string;
  recordingCount: number;
  evidenceCount: number;
  summary: string | null;
  reviewed: boolean;
  status:
    | 'empty'
    | 'pending'
    | 'ready'
    | 'stale'
    | 'failed'
    | 'unconfigured'
    | 'generating'
    | 'partial'
    | 'invalidated'
    | 'insufficient';
  inputFingerprint: string;
  capturedDurationMs: number;
  thumbnailPath: string | null;
  thumbnailRecordingId: string | null;
  redactionCount: number;
  recordingActive: boolean;
}
export interface RecallSession {
  id: string;
  title: string;
  category: string;
  titleCorrection: string | null;
  categoryCorrection: string | null;
  summary: string;
  startMs: number;
  endMs: number;
  citations: RecallCitation[];
  correction: string | null;
}
export interface RecallSuggestion {
  id: string;
  title: string;
  description: string;
  citations: RecallCitation[];
  acceptedTaskId: string | null;
  dismissed: boolean;
}
export interface RecallTask {
  id: string;
  dayId: string;
  title: string;
  notes: string;
  status: 'open' | 'done' | 'dismissed';
  citations: RecallCitation[];
  sourceWithheld: boolean;
  updatedAtMs: number;
}
export interface RecallFeedback {
  targetId: string;
  rating: 'helpful' | 'unhelpful' | null;
  comment: string;
}
export interface RecallCoverageInterval {
  /** Empty means all recording sources; redactions may affect selected tracks only. */
  sourceTrackIds: string[];
  recordingId: string;
  segmentId: string | null;
  startMs: number;
  endMs: number;
  kind: 'captured' | 'gap' | 'redacted' | 'missing' | 'uninterpreted';
}
export interface RecallDay {
  evidence?: RecallEvidence[];
  privacyRevision: number;
  day: RecallDayCard;
  recordings: RecallDayRecording[];
  sessions: RecallSession[];
  suggestions: RecallSuggestion[];
  tasks: RecallTask[];
  feedback: RecallFeedback[];
  coverage: RecallCoverageInterval[];
}
export interface RecallDayRecording {
  recordingId: string;
  title: string | null;
  startMs: number;
  endMs: number | null;
  startOffsetMs: number | null;
  durationMs: number | null;
  availability: 'available' | 'partial' | 'unavailable' | 'hidden';
  processingStatus: string | null;
}
export interface ListRecallDaysInput {
  beforeDayId: string | null;
  limit: number;
  timezone: string;
}
export interface RecallDayInput {
  dayId: string;
  timezone: string;
}
export interface ResolveRecallEvidenceInput {
  evidenceId: string;
}
export interface SetRecallFeedbackInput extends RecallFeedback {
  dayId: string;
}
export interface SetRecallSessionCorrectionInput {
  dayId: string;
  sessionId: string;
  correction: string | null;
  titleCorrection?: string | null;
  categoryCorrection?: string | null;
}
export interface DismissRecallSuggestionInput {
  dayId: string;
  suggestionId: string;
}
export interface AcceptRecallSuggestionInput {
  dayId: string;
  suggestionId: string;
}
export interface UpdateRecallTaskInput {
  taskId: string;
  title: string;
  notes: string;
  status: RecallTask['status'];
}
export interface SetRecallDayReviewedInput {
  dayId: string;
  reviewed: boolean;
  inputFingerprint: string;
}
export interface RecallJob {
  id: string;
  dayId: string | null;
  kind: 'index' | 'generation';
  status: RecallJobStatus;
  progress: number;
  errorCode: string | null;
  updatedAtMs: number;
}
export interface ControlRecallJobInput {
  jobId: string;
  action: 'pause' | 'resume' | 'cancel' | 'retry';
}
export interface RecallStatus {
  privacyRevision: number;
  indexedEvidenceCount: number;
  pendingEvidenceCount: number;
  embeddingReady: boolean;
  generationReady: boolean;
  embeddingUnavailableReason: string | null;
  generationUnavailableReason: string | null;
  jobs: RecallJob[];
}
export interface RecallStateChanged {
  dayId: string | null;
  privacyRevision: number;
}
export interface RecallExternalProfile {
  enabled: boolean;
  baseUrl: string | null;
  model: string | null;
  authMode: 'api-key' | 'none';
  apiKeyRef: string | null;
}
export interface RecallEmbeddingSettings extends RecallExternalProfile {
  provider: 'local' | 'openai-compatible';
  localModelId: 'multilingual-e5-small-q8_0' | 'bge-small-en-v1.5-q8_0';
  localBackend: 'cpu' | 'auto' | 'vulkan';
  dimensions: number;
  convention: 'e5' | 'bge' | 'plain';
}
export interface RecallSettings {
  embedding: RecallEmbeddingSettings;
  generation: RecallExternalProfile;
  autoGenerate: boolean;
}
export interface TestRecallProviderInput {
  kind: 'embedding' | 'generation';
  settings: RecallSettings;
  keyDraft: string | null;
}
export interface RecallProviderTestResult {
  kind: 'embedding' | 'generation';
  status: 'succeeded' | 'failed';
  errorCode: string | null;
  elapsedMs: number;
}
