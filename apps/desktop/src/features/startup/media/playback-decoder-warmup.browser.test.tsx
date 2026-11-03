import { act, cleanup, render, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import source from './decoder-warmup.mp4?url&no-inline';
import { PlaybackDecoderWarmup } from './playback-decoder-warmup';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: native.invoke }));
beforeEach(() => {
  native.invoke.mockReset();
  native.invoke.mockResolvedValue(new URL(source, location.href).href);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});
test('the packaged synthetic frame decodes and releases its media resource', async () => {
  const create = vi.spyOn(document, 'createElement');
  render(<PlaybackDecoderWarmup />);
  const video = create.mock.results
    .map((result) => result.value)
    .find((value) => value instanceof HTMLVideoElement);
  if (!(video instanceof HTMLVideoElement))
    throw new Error('Warmup video missing');
  const decoded = vi.fn();
  video.addEventListener('loadeddata', decoded);
  await waitFor(() => expect(decoded).toHaveBeenCalledOnce());
  expect(video.hasAttribute('src')).toBe(false);
  expect(video.paused).toBe(true);
});
test.each(['error', 'unmount', 'late lookup failure'])(
  'warmup releases resources after %s without blocking startup',
  async (mode) => {
    let finish: (url: string) => void = () => {};
    let reject: (error: Error) => void = () => {};
    if (mode === 'error')
      native.invoke.mockResolvedValueOnce(
        new URL('/__missing-warmup__.mp4', location.href).href,
      );
    else
      native.invoke.mockImplementationOnce(
        () =>
          new Promise<string>((resolve, fail) => {
            finish = resolve;
            reject = fail;
          }),
      );
    const create = vi.spyOn(document, 'createElement');
    const view = render(<PlaybackDecoderWarmup />);
    const video = create.mock.results
      .map((result) => result.value)
      .find((value) => value instanceof HTMLVideoElement);
    if (!(video instanceof HTMLVideoElement))
      throw new Error('Warmup video missing');
    if (mode === 'error') {
      await waitFor(() =>
        expect(
          native.invoke.mock.calls.some(
            ([command]) => command === 'log_frontend_failure',
          ),
        ).toBe(true),
      );
    } else {
      view.unmount();
      await act(async () => {
        if (mode === 'unmount') finish(new URL(source, location.href).href);
        else reject(new Error('Synthetic retired lookup failure'));
      });
      expect(native.invoke).not.toHaveBeenCalledWith(
        'log_frontend_failure',
        expect.objectContaining({
          input: expect.objectContaining({ stage: 'execute' }),
        }),
      );
    }
    expect(video.hasAttribute('src')).toBe(false);
  },
);

test('a decoded synthetic frame releases even when canvas is unavailable', async () => {
  const context = vi
    .spyOn(HTMLCanvasElement.prototype, 'getContext')
    .mockReturnValue(null);
  const create = vi.spyOn(document, 'createElement');
  render(<PlaybackDecoderWarmup />);
  const video = create.mock.results
    .map((result) => result.value)
    .find((value) => value instanceof HTMLVideoElement);
  if (!(video instanceof HTMLVideoElement))
    throw new Error('Warmup video missing');
  await waitFor(() => expect(context).toHaveBeenCalled());
  expect(video.hasAttribute('src')).toBe(false);
});
