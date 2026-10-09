import type { RecorderSceneSource } from '@daydream/shared';
import { expect, test } from 'vitest';
import {
  cropGesture,
  previewDemandSize,
  sourcePoint,
} from './crop-editor-geometry';
import { cropSource, resizeSource } from './source-capture-geometry';

test('source coordinates include fit, letterboxing, zoom and pan without applying DPR twice', () => {
  expect(
    sourcePoint(
      { x: 400, y: 200 },
      { left: 80, top: 20, scale: 0.25, zoom: 2, panX: 20, panY: 30 },
      { width: 5120, height: 1440 },
    ),
  ).toEqual({ x: 600, y: 300 });
});
test('moves and resize handles stay inside the source and preserve a two pixel minimum', () => {
  const bounds = { width: 5120, height: 1440 };
  const crop = { x: 100, y: 200, width: 800, height: 600 };
  expect(
    cropGesture('move', crop, { x: 100, y: 200 }, { x: -500, y: 1800 }, bounds),
  ).toEqual({ x: 0, y: 840, width: 800, height: 600 });
  expect(
    cropGesture('nw', crop, { x: 100, y: 200 }, { x: 1000, y: 1000 }, bounds),
  ).toEqual({ x: 898, y: 798, width: 2, height: 2 });
  expect(
    cropGesture('draw', crop, { x: 600, y: 500 }, { x: 200, y: 100 }, bounds),
  ).toEqual({ x: 200, y: 100, width: 400, height: 400 });
});
test('preview demand respects source size, DPR, view zoom and both memory caps', () => {
  const size = previewDemandSize(
    { width: 5120, height: 1440 },
    1280,
    360,
    2,
    2,
  );
  expect(size.width).toBeLessThanOrEqual(3840);
  expect(size.width * size.height).toBeLessThanOrEqual(4_000_000);
  expect(size.width / size.height).toBeCloseTo(5120 / 1440, 2);
  expect(
    previewDemandSize({ width: 800, height: 600 }, 1600, 1200, 2, 2),
  ).toEqual({ width: 800, height: 600 });
});
test('repeated integer crop resizing preserves the selected output scale across even rounding', () => {
  const source: RecorderSceneSource = {
    id: 'synthetic',
    sourceId: 'monitor',
    sourceKind: 'monitor',
    displayName: 'Synthetic',
    sourceWidth: 5120,
    sourceHeight: 1440,
    zIndex: 0,
    preserveAspectRatio: true,
    crop: { left: 0, top: 0, right: 0, bottom: 0 },
    rect: { x: 0, y: 0, width: 5120, height: 1440 },
  };
  let resized = resizeSource(source, 0.5);
  for (let width = 2001; width >= 1990; width--)
    resized = cropSource(resized, { x: 0, y: 0, width, height: 1440 });
  expect(resized.rect).toEqual({ x: 0, y: 0, width: 994, height: 720 });
});
