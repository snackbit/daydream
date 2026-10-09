import type { RecorderPreviewFrame } from '@daydream/shared';
import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { useDecodedPreview } from './use-decoded-preview';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
test('preview decoding drops queued stale frames and fences a previous source generation', async () => {
  const images: { url: string; finish: () => void }[] = [];
  class ControlledImage {
    src = '';
    decode() {
      return new Promise<void>((resolve) =>
        images.push({ url: this.src, finish: resolve }),
      );
    }
  }
  vi.stubGlobal('Image', ControlledImage);
  const frame = (
    generation: number,
    sequence: number,
  ): RecorderPreviewFrame => ({
    generation,
    sequence,
    sceneSourceId: 'synthetic',
    sourceId: 'monitor',
    timestampMs: sequence,
    width: 1,
    height: 1,
    dataUrl: `synthetic:${generation}:${sequence}`,
  });
  const view = renderHook(
    ({ generation, sequence }) =>
      useDecodedPreview(frame(generation, sequence), generation),
    { initialProps: { generation: 1, sequence: 1 } },
  );
  view.rerender({ generation: 1, sequence: 2 });
  view.rerender({ generation: 1, sequence: 3 });
  view.rerender({ generation: 2, sequence: 4 });
  expect(images).toHaveLength(1);
  await act(async () => images[0]!.finish());
  expect(view.result.current.decoded).toBeNull();
  expect(images.map((image) => image.url)).toEqual([
    'synthetic:1:1',
    'synthetic:2:4',
  ]);
  await act(async () => images[1]!.finish());
  await waitFor(() =>
    expect(view.result.current.decoded?.url).toBe('synthetic:2:4'),
  );
});
