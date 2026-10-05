import type {
  ActivityEventKind,
  ActivityIntervalState,
  ActivitySourceKind,
  RecordingTranscriptionChunkResult,
  TranscriptSourceRole,
} from '@daydream/shared';

export interface StudioActivityIntervalSelection {
  endOffsetMs: number;
  kind: 'activity-interval';
  score: number | null;
  sourceKind: ActivitySourceKind;
  startOffsetMs: number;
  state: ActivityIntervalState;
  timelineOffsetMs: number;
  trackId: string;
  trackLabel: string;
}

export interface StudioActivityEventSelection {
  applicationName: string | null;
  kind: 'activity-event';
  offsetMs: number;
  occurredAtMs: number;
  eventKind: ActivityEventKind;
  timelineOffsetMs: number;
  title: string | null;
  trackId: string;
  trackLabel: string;
}

export interface StudioTranscriptSegmentSelection {
  chunkResult: RecordingTranscriptionChunkResult;
  confidence: number | null;
  endOffsetMs: number;
  kind: 'transcript-segment';
  model: string | null;
  provider: string;
  segmentId: string;
  sourceRole: TranscriptSourceRole;
  startOffsetMs: number;
  text: string;
  timelineOffsetMs: number;
  trackId: string;
  trackLabel: string;
}

export type StudioInspectorSelection =
  | StudioActivityEventSelection
  | StudioActivityIntervalSelection
  | StudioTranscriptSegmentSelection;
