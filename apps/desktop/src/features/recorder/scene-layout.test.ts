import type { CaptureSource, RecorderScene } from '@daydream/shared';
import { expect, test } from 'vitest';
import {
  createDefaultRecorderScene,
  expandCanvasToFitSources,
  lowerSceneSource,
  mergeCaptureSourcesIntoScene,
  moveSceneSource,
  raiseSceneSource,
  removeSceneSource,
  resizeSceneSource,
  setSceneSourceAspectLock,
  setSceneSourceCrop,
  setSceneSourceRect,
  sourceDimensions,
} from './scene-layout';

const monitor = (
  id: string,
  dimensions: Partial<
    Pick<CaptureSource, 'height' | 'logicalHeight' | 'logicalWidth' | 'width'>
  >,
): CaptureSource => ({
  displayName: id,
  height: dimensions.height ?? null,
  id,
  kind: 'monitor',
  logicalHeight: dimensions.logicalHeight ?? null,
  logicalWidth: dimensions.logicalWidth ?? null,
  scaleFactor: null,
  width: dimensions.width ?? null,
});

test('sourceDimensions prefers physical pixels', () => {
  expect(
    sourceDimensions(
      monitor('main', {
        height: 1080,
        logicalHeight: 720,
        logicalWidth: 1280,
        width: 1920,
      }),
    ),
  ).toEqual({
    height: 1080,
    source: 'physical',
    width: 1920,
  });
});

test('sourceDimensions falls back to logical portal dimensions', () => {
  expect(
    sourceDimensions(
      monitor('scaled', {
        logicalHeight: 900,
        logicalWidth: 1440,
      }),
    ),
  ).toEqual({
    height: 900,
    source: 'logical',
    width: 1440,
  });
});

test('createDefaultRecorderScene lays monitors out horizontally', () => {
  const scene = createDefaultRecorderScene([
    monitor('left', { height: 1080, width: 1920 }),
    monitor('right', { height: 1440, width: 2560 }),
  ]);

  expect(scene).toMatchObject({
    canvasHeight: 1440,
    canvasWidth: 4480,
    layoutMode: 'horizontal',
  });
  expect(scene.id).toHaveLength(36);
  expect(scene.sources.map((source) => source.id)).toEqual([
    expect.stringMatching(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
    ),
    expect.stringMatching(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
    ),
  ]);
  expect(scene.sources.map((source) => source.rect)).toEqual([
    { height: 1080, width: 1920, x: 0, y: 0 },
    { height: 1440, width: 2560, x: 1920, y: 0 },
  ]);
});

test('createDefaultRecorderScene uses a deterministic unknown-dimension fallback', () => {
  const scene = createDefaultRecorderScene([monitor('unknown', {})]);

  expect(scene.canvasHeight).toBe(720);
  expect(scene.canvasWidth).toBe(1280);
  expect(scene.sources[0]?.rect).toEqual({
    height: 720,
    width: 1280,
    x: 0,
    y: 0,
  });
});

test('moveSceneSource clamps movement to the canvas', () => {
  const scene = createDefaultRecorderScene([
    monitor('main', { height: 1080, width: 1920 }),
  ]);
  const sourceId = sceneSourceId(scene);

  const moved = moveSceneSource(scene, sourceId, 200, 100);

  expect(moved.sources[0]?.rect).toEqual({
    height: 1080,
    width: 1920,
    x: 0,
    y: 0,
  });
});

test('resizeSceneSource preserves aspect ratio by default', () => {
  const scene = createDefaultRecorderScene([
    monitor('main', { height: 1080, width: 1920 }),
  ]);
  const sourceId = sceneSourceId(scene);

  const resized = resizeSceneSource(scene, sourceId, 'se', -960, -10);

  expect(resized.sources[0]?.rect.width).toBe(960);
  expect(resized.sources[0]?.rect.height).toBe(540);
});

test('resizeSceneSource respects aspect ratio from side handles', () => {
  const scene = createDefaultRecorderScene([
    monitor('main', { height: 1080, width: 1920 }),
  ]);
  const sourceId = sceneSourceId(scene);

  const eastResized = resizeSceneSource(scene, sourceId, 'e', -960, 0);
  expect(eastResized.sources[0]?.rect).toEqual({
    height: 540,
    width: 960,
    x: 0,
    y: 0,
  });

  const southResized = resizeSceneSource(scene, sourceId, 's', 0, -540);
  expect(southResized.sources[0]?.rect).toEqual({
    height: 540,
    width: 960,
    x: 0,
    y: 0,
  });
});

test('resizeSceneSource anchors north and west handles while preserving aspect ratio', () => {
  const scene = createDefaultRecorderScene([
    monitor('main', { height: 1080, width: 1920 }),
  ]);
  const sourceId = sceneSourceId(scene);

  const westResized = resizeSceneSource(scene, sourceId, 'w', 960, 0);
  expect(westResized.sources[0]?.rect).toEqual({
    height: 540,
    width: 960,
    x: 960,
    y: 0,
  });

  const northResized = resizeSceneSource(scene, sourceId, 'n', 0, 540);
  expect(northResized.sources[0]?.rect).toEqual({
    height: 540,
    width: 960,
    x: 0,
    y: 540,
  });
});

test('resizeSceneSource clamps max-clamped corner resize to the canvas', () => {
  const scene = createDefaultRecorderScene([
    monitor('main', { height: 1080, width: 1920 }),
  ]);
  const sourceId = sceneSourceId(scene);

  const resized = resizeSceneSource(scene, sourceId, 'se', 500, 500);

  expect(resized.sources[0]?.rect).toEqual({
    height: 1080,
    width: 1920,
    x: 0,
    y: 0,
  });
});

test('resizeSceneSource can resize without aspect lock', () => {
  const initialScene = createDefaultRecorderScene([
    monitor('main', { height: 1080, width: 1920 }),
  ]);
  const sourceId = sceneSourceId(initialScene);
  const scene = setSceneSourceAspectLock(initialScene, sourceId, false);

  const resized = resizeSceneSource(scene, sourceId, 'se', -960, -80);

  expect(resized.sources[0]?.rect.width).toBe(960);
  expect(resized.sources[0]?.rect.height).toBe(1000);
});

test('resizeSceneSource keeps west edge resize anchored when clamped to minimum width', () => {
  const initialScene = createDefaultRecorderScene([
    monitor('main', { height: 100, width: 200 }),
  ]);
  const sourceId = sceneSourceId(initialScene);
  const scene = setSceneSourceAspectLock(
    {
      ...initialScene,
      canvasWidth: 500,
      sources: [
        {
          ...sceneSource(initialScene),
          rect: {
            height: 100,
            width: 200,
            x: 100,
            y: 0,
          },
        },
      ],
    },
    sourceId,
    false,
  );

  const resized = resizeSceneSource(scene, sourceId, 'w', 300, 0);

  expect(resized.sources[0]?.rect).toEqual({
    height: 100,
    width: 64,
    x: 236,
    y: 0,
  });
});

test('resizeSceneSource keeps north edge resize anchored when clamped to minimum height', () => {
  const initialScene = createDefaultRecorderScene([
    monitor('main', { height: 120, width: 200 }),
  ]);
  const sourceId = sceneSourceId(initialScene);
  const scene = setSceneSourceAspectLock(
    {
      ...initialScene,
      canvasHeight: 300,
      sources: [
        {
          ...sceneSource(initialScene),
          rect: {
            height: 120,
            width: 200,
            x: 0,
            y: 80,
          },
        },
      ],
    },
    sourceId,
    false,
  );

  const resized = resizeSceneSource(scene, sourceId, 'n', 0, 200);

  expect(resized.sources[0]?.rect).toEqual({
    height: 36,
    width: 200,
    x: 0,
    y: 164,
  });
});

test('setSceneSourceCrop clamps impossible crop values', () => {
  const scene = createDefaultRecorderScene([
    monitor('main', { height: 100, width: 200 }),
  ]);
  const sourceId = sceneSourceId(scene);

  const cropped = setSceneSourceCrop(scene, sourceId, {
    bottom: 90,
    left: 150,
    right: 100,
    top: 90,
  });

  expect(cropped.sources[0]?.crop).toEqual({
    bottom: 9,
    left: 150,
    right: 49,
    top: 90,
  });
});

test('resizeSceneSource re-clamps existing crop to resized dimensions', () => {
  const initialScene = createDefaultRecorderScene([
    monitor('main', { height: 100, width: 200 }),
  ]);
  const sourceId = sceneSourceId(initialScene);
  const unlockedScene = setSceneSourceAspectLock(initialScene, sourceId, false);
  const croppedScene = setSceneSourceCrop(unlockedScene, sourceId, {
    bottom: 0,
    left: 150,
    right: 0,
    top: 90,
  });

  const resized = resizeSceneSource(croppedScene, sourceId, 'se', -300, -200);

  expect(resized.sources[0]?.rect).toEqual({
    height: 36,
    width: 64,
    x: 0,
    y: 0,
  });
  expect(resized.sources[0]?.crop).toEqual({
    bottom: 0,
    left: 63,
    right: 0,
    top: 35,
  });
});

test('setSceneSourceRect re-clamps existing crop to resized dimensions', () => {
  const initialScene = createDefaultRecorderScene([
    monitor('main', { height: 100, width: 200 }),
  ]);
  const sourceId = sceneSourceId(initialScene);
  const croppedScene = setSceneSourceCrop(initialScene, sourceId, {
    bottom: 0,
    left: 150,
    right: 0,
    top: 90,
  });

  const resized = setSceneSourceRect(croppedScene, sourceId, {
    height: 36,
    width: 64,
    x: 0,
    y: 0,
  });

  expect(resized.sources[0]?.crop).toEqual({
    bottom: 0,
    left: 63,
    right: 0,
    top: 35,
  });
});

test('z-order helpers move a source above and below peers', () => {
  const scene = createDefaultRecorderScene([
    monitor('left', { height: 1080, width: 1920 }),
    monitor('right', { height: 1080, width: 1920 }),
  ]);
  const leftSourceId = sceneSourceId(scene);

  const raised = raiseSceneSource(scene, leftSourceId);
  expect(raised.sources[0]?.zIndex).toBe(2);

  const lowered = lowerSceneSource(raised, leftSourceId);
  expect(lowered.sources[0]?.zIndex).toBe(-1);
});

test('mergeCaptureSourcesIntoScene preserves existing geometry and appends new sources', () => {
  const initialScene = createDefaultRecorderScene([
    monitor('main', { height: 900, width: 1440 }),
  ]);
  const sourceId = sceneSourceId(initialScene);
  const editedScene = setSceneSourceAspectLock(
    moveSceneSource(
      {
        ...initialScene,
        canvasWidth: 2400,
      },
      sourceId,
      120,
      0,
    ),
    sourceId,
    false,
  );

  const merged = mergeCaptureSourcesIntoScene(editedScene, [
    monitor('main', { height: 900, width: 1440 }),
    monitor('secondary', { height: 720, width: 1280 }),
  ]);

  expect(merged.sources).toHaveLength(2);
  expect(merged.sources[0]).toEqual(editedScene.sources[0]);
  expect(merged.sources[1]?.sourceId).toBe('secondary');
  expect(merged.sources[1]?.rect).toEqual({
    height: 720,
    width: 1280,
    x: 1560,
    y: 0,
  });
  expect(merged.canvasWidth).toBe(2840);
  expect(merged.layoutMode).toBe('custom');
});

test('mergeCaptureSourcesIntoScene ignores duplicate source ids', () => {
  const scene = createDefaultRecorderScene([
    monitor('main', { height: 900, width: 1440 }),
  ]);

  const merged = mergeCaptureSourcesIntoScene(scene, [
    monitor('main', { height: 900, width: 1440 }),
  ]);

  expect(merged).toBe(scene);
});

test('mergeCaptureSourcesIntoScene ignores duplicate source ids inside new sources', () => {
  const scene = createDefaultRecorderScene([]);

  const merged = mergeCaptureSourcesIntoScene(scene, [
    monitor('main', { height: 900, width: 1440 }),
    monitor('main', { height: 900, width: 1440 }),
  ]);

  expect(merged.sources.map((source) => source.sourceId)).toEqual(['main']);
});

test('removeSceneSource removes by scene source id and normalizes z-order gaps', () => {
  const scene = createDefaultRecorderScene([
    monitor('left', { height: 900, width: 1440 }),
    monitor('right', { height: 900, width: 1440 }),
    monitor('top', { height: 900, width: 1440 }),
  ]);
  const removed = removeSceneSource(scene, sceneSourceId(scene, 1));

  expect(removed.sources.map((source) => source.sourceId)).toEqual([
    'left',
    'top',
  ]);
  expect(removed.sources.map((source) => source.zIndex)).toEqual([0, 1]);
  expect(removed.layoutMode).toBe('custom');
});

test('expandCanvasToFitSources only grows the canvas', () => {
  const scene = createDefaultRecorderScene([
    monitor('main', { height: 900, width: 1440 }),
  ]);
  const sourceId = sceneSourceId(scene);
  const moved = {
    ...setSceneSourceAspectLock(scene, sourceId, false),
    canvasHeight: 800,
    canvasWidth: 800,
    sources: [
      {
        ...sceneSource(scene),
        rect: {
          height: 900,
          width: 1440,
          x: 100,
          y: 50,
        },
      },
    ],
  };

  const expanded = expandCanvasToFitSources(moved);

  expect(expanded.canvasWidth).toBe(1540);
  expect(expanded.canvasHeight).toBe(950);
});

function sceneSourceId(scene: RecorderScene, index = 0): string {
  return sceneSource(scene, index).id;
}

function sceneSource(scene: RecorderScene, index = 0) {
  const source = scene.sources[index];
  if (!source) {
    throw new Error(`Expected scene source at index ${index}.`);
  }

  return source;
}
