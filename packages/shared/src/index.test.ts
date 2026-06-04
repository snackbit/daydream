import { expect, test } from 'vitest';
import type { DaydreamDiagnostics } from './index.js';

test('diagnostics shape supports unavailable ffmpeg state', () => {
  const diagnostics: DaydreamDiagnostics = {
    app: {
      identifier: 'com.snackbit.daydream',
      name: 'Daydream',
      version: '0.1.0',
    },
    ffmpeg: {
      available: false,
      error: 'missing DAYDREAM_FFMPEG_PATH',
      path: null,
      source: 'unconfigured',
      versionLine: null,
    },
    storage: {
      appDataDir: null,
    },
    system: {
      arch: 'x86_64',
      family: 'unix',
      platform: 'linux',
    },
  };

  expect(diagnostics.ffmpeg.available).toBe(false);
});
