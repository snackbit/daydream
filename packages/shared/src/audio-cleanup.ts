/** Local, reversible processing of the microphone using desktop audio. */
export interface AudioCleanupConfig {
  mode: 'off' | 'auto';
  matchThreshold: number;
  amount: number;
}

export type AudioCleanupPhase =
  'idle' | 'running' | 'ready' | 'cancelled' | 'failed' | 'invalidated';

export interface AudioCleanupState {
  recordingId: string;
  config: AudioCleanupConfig;
  pendingConfig: AudioCleanupConfig | null;
  revision: string | null;
  phase: AudioCleanupPhase;
  progress: number;
  matchedMs: number;
  referenceAvailable: boolean;
  transcriptsOutdated: boolean;
  error: string | null;
}

export interface ApplyAudioCleanupInput {
  recordingId: string;
  config: AudioCleanupConfig;
}

export interface PreviewAudioCleanupInput extends ApplyAudioCleanupInput {
  startOffsetMs: number;
  endOffsetMs: number;
}

export interface AudioCleanupPreview {
  recordingId: string;
  sessionId: string;
  originalUrl: string;
  cleanedUrl: string;
  matchedMs: number;
  durationMs: number;
}
