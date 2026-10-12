import type {
  ApplyAudioCleanupInput,
  AudioCleanupConfig,
  AudioCleanupPhase,
  AudioCleanupPreview,
  AudioCleanupState,
  PreviewAudioCleanupInput,
} from '@daydream/shared';
import {
  boolean,
  count,
  invalid,
  nullable,
  record,
  string,
} from '@/features/processing/task-decoder';
import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';

function fraction(value: unknown): number {
  return typeof value === 'number' &&
    Number.isFinite(value) &&
    value >= 0 &&
    value <= 1
    ? value
    : invalid();
}
function config(value: unknown): AudioCleanupConfig {
  const row = record(value);
  if (row.mode !== 'off' && row.mode !== 'auto') return invalid();
  const matchThreshold = fraction(row.matchThreshold);
  if (matchThreshold < 0.1 || matchThreshold > 0.99) return invalid();
  return { mode: row.mode, matchThreshold, amount: fraction(row.amount) };
}
function phase(value: unknown): AudioCleanupPhase {
  if (
    value === 'idle' ||
    value === 'running' ||
    value === 'ready' ||
    value === 'cancelled' ||
    value === 'failed' ||
    value === 'invalidated'
  )
    return value;
  return invalid();
}
export function decodeAudioCleanupState(value: unknown): AudioCleanupState {
  const row = record(value);
  return {
    recordingId: string(row.recordingId),
    config: config(row.config),
    pendingConfig: nullable(row.pendingConfig, config),
    revision: nullable(row.revision, string),
    phase: phase(row.phase),
    progress: fraction(row.progress),
    matchedMs: count(row.matchedMs),
    referenceAvailable: boolean(row.referenceAvailable),
    transcriptsOutdated: boolean(row.transcriptsOutdated),
    error: nullable(row.error, string),
  };
}
export function decodeAudioCleanupPreview(value: unknown): AudioCleanupPreview {
  const row = record(value);
  const url = (value: unknown) => {
    const text = string(value);
    const url = new URL(text);
    if (
      url.protocol !== 'http:' ||
      url.hostname !== '127.0.0.1' ||
      !url.port ||
      !url.pathname.startsWith('/playback/')
    )
      return invalid();
    return text;
  };
  const durationMs = count(row.durationMs);
  if (durationMs === 0 || durationMs > 15_000) return invalid();
  return {
    recordingId: string(row.recordingId),
    sessionId: string(row.sessionId),
    originalUrl: url(row.originalUrl),
    cleanedUrl: url(row.cleanedUrl),
    matchedMs: count(row.matchedMs),
    durationMs,
  };
}
export async function getAudioCleanup(
  recordingId: string,
): Promise<AudioCleanupState> {
  return decodeResponse(
    'getAudioCleanup',
    await invoke<unknown>('getAudioCleanup', { recordingId }),
    decodeAudioCleanupState,
  );
}
export async function applyAudioCleanup(
  input: ApplyAudioCleanupInput,
): Promise<AudioCleanupState> {
  return decodeResponse(
    'applyAudioCleanup',
    await invoke<unknown>('applyAudioCleanup', { input }),
    decodeAudioCleanupState,
  );
}
export async function cancelAudioCleanup(
  recordingId: string,
): Promise<AudioCleanupState> {
  return decodeResponse(
    'cancelAudioCleanup',
    await invoke<unknown>('cancelAudioCleanup', { recordingId }),
    decodeAudioCleanupState,
  );
}
export async function previewAudioCleanup(
  input: PreviewAudioCleanupInput,
): Promise<AudioCleanupPreview> {
  return decodeResponse(
    'previewAudioCleanup',
    await invoke<unknown>('previewAudioCleanup', { input }),
    decodeAudioCleanupPreview,
  );
}
