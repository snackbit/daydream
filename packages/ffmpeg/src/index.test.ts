import { expect, test } from 'vitest';
import { resolveFfmpeg } from './index.js';

test('resolves a development FFmpeg binary', () => {
  const resolution = resolveFfmpeg();

  expect(resolution.source).toBe('ffmpeg-static');
  expect(resolution.path).toContain('ffmpeg');
});
