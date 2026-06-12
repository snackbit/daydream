import type {
  RecordingTranscript,
  RecordingTranscriptionTrackState,
} from './transcript.js';

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
  resultCode: string | null;
  segmentId: string | null;
  sourceKind: ProcessingChunkSourceKind;
  startOffsetMs: number | null;
  status: ProcessingChunkStatus;
  trackId: string | null;
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

export interface GetRecordingAiProcessingStateInput {
  recordingId: string;
}

export interface RecordingAiProcessingState {
  chunks: ProcessingChunk[];
  job: ProcessingJob | null;
  recordingId: string;
  transcript: RecordingTranscript;
  transcriptionTracks?: RecordingTranscriptionTrackState[];
}
