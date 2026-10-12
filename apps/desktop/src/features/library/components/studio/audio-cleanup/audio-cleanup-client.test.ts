import { expect, test } from 'vitest';
import {
  decodeAudioCleanupPreview,
  decodeAudioCleanupState,
} from './audio-cleanup-client';

const state = {
  recordingId: 'synthetic',
  config: { mode: 'auto', matchThreshold: 0.65, amount: 1 },
  pendingConfig: null,
  revision: 'revision-1',
  phase: 'ready',
  progress: 1,
  matchedMs: 4000,
  referenceAvailable: true,
  transcriptsOutdated: false,
  error: null,
};

test('cleanup IPC accepts ready state and rejects invalid settings or lifecycle', () => {
  expect(decodeAudioCleanupState(state)).toEqual(state);
  for (const value of [
    { ...state, phase: 'finished' },
    { ...state, progress: Infinity },
    { ...state, config: { ...state.config, amount: 2 } },
    { ...state, config: { ...state.config, matchThreshold: 0.01 } },
  ]) {
    expect(() => decodeAudioCleanupState(value)).toThrow();
  }
});
test('comparison accepts only local authenticated playback URLs and bounded ranges', () => {
  const preview = {
    recordingId: 'synthetic',
    sessionId: 'session',
    originalUrl:
      'http://127.0.0.1:4321/playback/session/token/segments/original/audio/microphone-0',
    cleanedUrl:
      'http://127.0.0.1:4321/playback/session/token/segments/cleaned/audio/microphone-0',
    matchedMs: 3000,
    durationMs: 10_000,
  };
  expect(decodeAudioCleanupPreview(preview)).toEqual(preview);
  for (const value of [
    { ...preview, cleanedUrl: 'https://remote.example/audio' },
    { ...preview, durationMs: 0 },
    { ...preview, durationMs: 30_000 },
  ]) {
    expect(() => decodeAudioCleanupPreview(value)).toThrow();
  }
});
