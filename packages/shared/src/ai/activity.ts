import type {
  ActivityIntervalState,
  ActivitySourceKind,
} from './provider-kinds.js';

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
