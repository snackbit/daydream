import type {
  ProcessingChunkStatus,
  ProcessingJobStatus,
} from './processing.js';
import type { AiProviderKind, TranscriptSourceRole } from './provider-kinds.js';

export type RecordingTranscriptionChunkResult =
  | 'completed-with-errors'
  | 'cancelled'
  | 'failed'
  | 'no-speech'
  | 'queued'
  | 'running'
  | 'skipped-silence'
  | 'text'
  | 'unknown';

export type RecordingTranscriptionTrackStatus =
  | 'cancelled'
  | 'complete'
  | 'failed'
  | 'in-progress'
  | 'no-audio'
  | 'no-speech'
  | 'ready';

export interface GetRecordingTranscriptInput {
  recordingId: string;
}

export interface GetRecordingTranscriptPreviewInput {
  recordingId: string;
  sourceRole: TranscriptSourceRole;
  startOffsetMs: number;
  endOffsetMs: number;
}

export interface TranscriptPreviewEntry {
  id: string;
  segmentId: string;
  startOffsetMs: number;
  endOffsetMs: number;
  text: string;
  truncated: boolean;
}

export interface RecordingTranscriptPreview {
  recordingId: string;
  sourceRole: TranscriptSourceRole;
  entries: TranscriptPreviewEntry[];
}

/** Source-time coverage published atomically with the current transcript. */
export interface RecordingTranscriptionProgress {
  jobId: string;
  trackId: string;
  sourceRole: TranscriptSourceRole;
  status: ProcessingJobStatus;
  ranges: Array<{ startOffsetMs: number; endOffsetMs: number }>;
  publishedRanges: Array<{ startOffsetMs: number; endOffsetMs: number }>;
}

export interface RecordingTranscriptionTrackState {
  chunks: RecordingTranscriptionChunkState[];
  description: string;
  label: string;
  libraryTrackId: string | null;
  sourceRole: TranscriptSourceRole;
  status: RecordingTranscriptionTrackStatus;
  statusLabel: string;
  textSegmentCount: number;
  trackId: string | null;
  trackKey: string;
}

export interface RecordingTranscriptionChunkState {
  attemptCount: number;
  endOffsetMs: number | null;
  errorCode: string | null;
  id: string;
  progress: number;
  result: RecordingTranscriptionChunkResult;
  resultCode: string | null;
  segmentId: string | null;
  startOffsetMs: number | null;
  status: ProcessingChunkStatus;
  text: string | null;
  transcriptSegments: RecordingTranscriptSegment[];
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
