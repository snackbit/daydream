import { beforeEach, expect, test, vi } from 'vitest';
import { getPlaybackDecoderWarmupUrl } from './playback-warmup-client';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: native.invoke }));
beforeEach(() => {
  native.invoke.mockReset();
});

test.each([
  'http://127.0.0.1:1234/decoder-warmup.mp4',
  'http://localhost:1234/decoder-warmup.mp4',
  'http://[::1]:1234/decoder-warmup.mp4',
])('accepts a loopback-only synthetic frame at %s', async (url) => {
  native.invoke.mockResolvedValueOnce(url);
  await expect(getPlaybackDecoderWarmupUrl()).resolves.toBe(url);
  expect(native.invoke).toHaveBeenCalledWith('getPlaybackDecoderWarmupUrl');
});
test.each([
  null,
  '/relative.mp4',
  'https://127.0.0.1/frame.mp4',
  'http://example.com/frame.mp4',
])('rejects an invalid or remote warmup source %s', async (value) => {
  native.invoke.mockResolvedValueOnce(value);
  await expect(getPlaybackDecoderWarmupUrl()).rejects.toThrow();
});
