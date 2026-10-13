import type { AiVisionProviderKind } from './provider-kinds.js';

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
  contextStartMs?: number | null;
  confirmedEndMs?: number | null;
  chunkId: string;
  confidence: number | null;
  frameTimestampMs: number;
  id: string;
  jobId: string;
  model: string | null;
  observationKind: 'corrected-ocr' | 'screen-observation';
  promptVersion: string;
  provider: AiVisionProviderKind;
  recordingId: string;
  region: VisualRegion;
  sourceFrameRef: string;
  text: string;
}

export type VlmProviderStatus =
  | 'configured'
  | 'disabled'
  | 'missing-base-url'
  | 'missing-key'
  | 'missing-model';

export type VlmStateStatus =
  | 'disabled'
  | 'empty'
  | 'failed'
  | 'not-planned'
  | 'queued'
  | 'running'
  | 'succeeded'
  | 'unconfigured';

export interface VlmState {
  errorCode: string | null;
  providerStatus: VlmProviderStatus;
  status: VlmStateStatus;
}

export interface RecordingVisualObservations {
  ocrRegions: OcrRegion[];
  recordingId: string;
  vlmState: VlmState;
  vlmObservations: VlmObservation[];
}
