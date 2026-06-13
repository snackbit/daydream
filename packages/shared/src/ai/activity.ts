import type {
  ActivityEventKind,
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
  captureStates: ActivityCaptureState[];
  events: ActivityEvent[];
  intervals: ActivityInterval[];
  recordingId: string;
}

export interface ActivityEvent {
  applicationName: string | null;
  id: string;
  kind: ActivityEventKind;
  occurredAtMs: number;
  offsetMs: number;
  recordingId: string;
  title: string | null;
}

export interface ActivityCaptureState {
  id: string;
  offsetMs: number;
  reason: string | null;
  recordingId: string;
  source: string | null;
  status: string | null;
}
