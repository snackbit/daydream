import type {
  CanvasRect,
  CaptureSource,
  RecorderScene,
} from '@daydream/shared';
import { expect, test } from 'vitest';
import {
  applySceneLayout,
  centerSceneSource,
  classifyCaptureSourceMerge,
  createDefaultRecorderScene,
  fillSceneSourceAvailableSpace,
  fillSceneSourceToCanvas,
  fitCanvasToSources,
  fitSceneSourceToCanvas,
  expandCanvasToFitSources,
  lowerSceneSource,
  mergeCaptureSourcesIntoScene,
  moveSceneSource,
  raiseSceneSource,
  removeSceneSource,
  resizeSceneSource,
  resolveDuplicateCaptureSource,
  setSceneCanvasSize,
  setSceneSourceAspectLock,
  setSceneSourceCrop,
  setSceneSourceRect,
  sceneSourceScale,
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

test('createDefaultRecorderScene uses auto 16:9 layout by default', () => {
  const scene = createDefaultRecorderScene([
    monitor('left', { height: 1080, width: 1920 }),
    monitor('right', { height: 1440, width: 2560 }),
  ]);

  expect(scene).toMatchObject({
    canvasHeight: 2880,
    canvasWidth: 2560,
    layoutMode: 'auto-16-9',
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
    { height: 1080, width: 1920, x: 320, y: 180 },
    { height: 1440, width: 2560, x: 0, y: 1440 },
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
  expect(scene.sources[0]?.sourceWidth).toBeNull();
  expect(scene.sources[0]?.sourceHeight).toBeNull();
});

test('createDefaultRecorderScene keeps a single ultrawide monitor at native auto size', () => {
  const scene = createDefaultRecorderScene([
    monitor('ultrawide', { height: 1440, width: 5120 }),
  ]);

  expect(scene.canvasWidth).toBe(5120);
  expect(scene.canvasHeight).toBe(1440);
  expect(scene.sources[0]?.rect).toEqual({
    height: 1440,
    width: 5120,
    x: 0,
    y: 0,
  });
  expect(scene.sources[0]?.sourceWidth).toBe(5120);
  expect(scene.sources[0]?.sourceHeight).toBe(1440);
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

test('resizeSceneSource uses native ultrawide aspect for locked sources', () => {
  const initialScene = createDefaultRecorderScene([
    monitor('ultrawide', { height: 1440, width: 5120 }),
  ]);
  const sourceId = sceneSourceId(initialScene);
  const scene = setSceneCanvasSize(initialScene, 2560, 1440);

  const resized = resizeSceneSource(scene, sourceId, 'e', -1280, 0);

  expect(resized.sources[0]?.rect).toEqual({
    height: 360,
    width: 1280,
    x: 0,
    y: 360,
  });
  expect(sceneSourceScale(resized.sources[0] ?? sceneSource(resized))).toEqual({
    scale: 0.25,
    scaleX: 0.25,
    scaleY: 0.25,
    sourceHeight: 1440,
    sourceWidth: 5120,
  });
});

test('sceneSourceScale reports separate width and height scale for distorted sources', () => {
  const initialScene = createDefaultRecorderScene([
    monitor('ultrawide', { height: 1440, width: 5120 }),
  ]);
  const sourceId = sceneSourceId(initialScene);
  const unlocked = setSceneSourceAspectLock(initialScene, sourceId, false);
  const distorted = resizeSceneSource(unlocked, sourceId, 'se', -2560, -360);

  expect(sceneSourceScale(sceneSource(distorted))).toEqual({
    scale: 0.5,
    scaleX: 0.5,
    scaleY: 0.75,
    sourceHeight: 1440,
    sourceWidth: 5120,
  });
});

test('setSceneSourceAspectLock relocks distorted sources to native aspect immediately', () => {
  const initialScene = createDefaultRecorderScene([
    monitor('ultrawide', { height: 1440, width: 5120 }),
  ]);
  const sourceId = sceneSourceId(initialScene);
  const scene = {
    ...setSceneSourceAspectLock(
      setSceneCanvasSize(initialScene, 2560, 1440),
      sourceId,
      false,
    ),
    sources: [
      {
        ...sceneSource(initialScene),
        preserveAspectRatio: false,
        rect: {
          height: 1000,
          width: 1000,
          x: 200,
          y: 100,
        },
      },
    ],
  };

  const relocked = setSceneSourceAspectLock(scene, sourceId, true);

  expect(relocked.sources[0]?.rect).toEqual({
    height: 720,
    width: 2560,
    x: 0,
    y: 240,
  });
  expect(relocked.sources[0]?.preserveAspectRatio).toBe(true);
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

test('resizeSceneSource keeps west edge resize anchored when clamped to canvas bounds', () => {
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

  const resized = resizeSceneSource(scene, sourceId, 'w', -1000, 0);

  expect(resized.sources[0]?.rect).toEqual({
    height: 100,
    width: 300,
    x: 0,
    y: 0,
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

test('setSceneSourceRect can preserve native aspect for geometry width and height edits', () => {
  const scene = sceneWithSingleSource({
    canvasHeight: 2880,
    canvasWidth: 5120,
    locked: true,
    rect: {
      height: 360,
      width: 1280,
      x: 0,
      y: 0,
    },
    sourceHeight: 1440,
    sourceWidth: 5120,
  });
  const sourceId = sceneSourceId(scene);

  const widthDriven = setSceneSourceRect(
    scene,
    sourceId,
    {
      height: 360,
      width: 2560,
      x: 0,
      y: 0,
    },
    { resizeDimension: 'width' },
  );
  expect(widthDriven.sources[0]?.rect).toEqual({
    height: 720,
    width: 2560,
    x: 0,
    y: 0,
  });

  const heightDriven = setSceneSourceRect(
    scene,
    sourceId,
    {
      height: 1080,
      width: 1280,
      x: 0,
      y: 0,
    },
    { resizeDimension: 'height' },
  );
  expect(heightDriven.sources[0]?.rect).toEqual({
    height: 1080,
    width: 3840,
    x: 0,
    y: 0,
  });

  const oversized = setSceneSourceRect(
    scene,
    sourceId,
    {
      height: 360,
      width: 10240,
      x: 0,
      y: 0,
    },
    { resizeDimension: 'width' },
  );
  expect(oversized.sources[0]?.rect).toEqual({
    height: 1440,
    width: 5120,
    x: 0,
    y: 0,
  });

  const exact = setSceneSourceRect(scene, sourceId, {
    height: 2880,
    width: 5120,
    x: 0,
    y: 0,
  });
  expect(exact.sources[0]?.rect).toEqual({
    height: 2880,
    width: 5120,
    x: 0,
    y: 0,
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

test('moveSceneSource allows manual overlap', () => {
  const scene = createDefaultRecorderScene([
    monitor('left', { height: 1080, width: 960 }),
    monitor('right', { height: 1080, width: 960 }),
  ]);

  const moved = moveSceneSource(scene, sceneSourceId(scene, 1), -960, 0);

  expect(moved.sources[1]?.rect).toEqual({
    height: 1080,
    width: 960,
    x: 0,
    y: 0,
  });
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

test('classifyCaptureSourceMerge reports likely duplicate monitor matches', () => {
  const scene = createDefaultRecorderScene([
    monitor('Main display', { height: 1440, width: 5120 }),
  ]);
  const duplicate = monitor('Main display', { height: 1440, width: 5120 });
  duplicate.id = 'main-new-portal-id';

  const classification = classifyCaptureSourceMerge(scene, [duplicate]);

  expect(classification.newSources).toHaveLength(0);
  expect(classification.duplicateCandidates).toHaveLength(1);
  expect(classification.duplicateCandidates[0]?.source.id).toBe(
    'main-new-portal-id',
  );
});

test('resolveDuplicateCaptureSource refreshes source id without changing geometry', () => {
  const scene = createDefaultRecorderScene([
    monitor('Main display', { height: 1440, width: 5120 }),
  ]);
  const existingSource = sceneSource(scene);
  const duplicate = monitor('Main display', { height: 1440, width: 5120 });
  duplicate.id = 'main-new-portal-id';

  const refreshed = resolveDuplicateCaptureSource(
    scene,
    {
      existingSource,
      source: duplicate,
    },
    'refresh-existing',
  );

  expect(refreshed.sources).toHaveLength(1);
  expect(refreshed.sources[0]).toEqual({
    ...existingSource,
    sourceId: 'main-new-portal-id',
  });
});

test('auto layout for two ultrawide sources chooses a 16:9-ish vertical grid', () => {
  const scene = createDefaultRecorderScene([
    monitor('left', { height: 1440, width: 5120 }),
    monitor('right', { height: 1440, width: 5120 }),
  ]);

  expect(scene.canvasWidth).toBe(5120);
  expect(scene.canvasHeight).toBe(2880);
  expect(scene.sources.map((source) => source.rect)).toEqual([
    { height: 1440, width: 5120, x: 0, y: 0 },
    { height: 1440, width: 5120, x: 0, y: 1440 },
  ]);
});

test('applySceneLayout supports horizontal vertical and auto arrangements', () => {
  const scene = createDefaultRecorderScene([
    monitor('left', { height: 900, width: 1440 }),
    monitor('right', { height: 900, width: 1440 }),
    monitor('top', { height: 900, width: 1440 }),
  ]);

  const horizontal = applySceneLayout(scene, 'horizontal');
  const vertical = applySceneLayout(scene, 'vertical');
  const auto = applySceneLayout(scene, 'auto-16-9');

  expect(horizontal.sources.map((source) => source.rect.x)).toEqual([
    0, 1440, 2880,
  ]);
  expect(vertical.sources.map((source) => source.rect.y)).toEqual([
    0, 900, 1800,
  ]);
  expect(auto.sources.map((source) => source.rect)).toEqual([
    { height: 900, width: 1440, x: 0, y: 0 },
    { height: 900, width: 1440, x: 1440, y: 0 },
    { height: 900, width: 1440, x: 0, y: 900 },
  ]);
});

test('setSceneCanvasSize reapplies non-custom layout after resizing canvas', () => {
  const scene = createDefaultRecorderScene([
    monitor('main', { height: 900, width: 1440 }),
  ]);
  const resized = setSceneCanvasSize(scene, 800, 450);

  expect(resized.layoutMode).toBe('auto-16-9');
  expect(resized.canvasWidth).toBe(800);
  expect(resized.canvasHeight).toBe(450);
  expect(resized.sources[0]?.rect).toEqual({
    height: 450,
    width: 720,
    x: 40,
    y: 0,
  });
});

test('setSceneCanvasSize clamps custom source geometry', () => {
  const scene = {
    ...createDefaultRecorderScene([
      monitor('main', { height: 900, width: 1440 }),
    ]),
    layoutMode: 'custom' as const,
  };
  const resized = setSceneCanvasSize(scene, 800, 450);

  expect(resized.layoutMode).toBe('custom');
  expect(resized.sources[0]?.rect).toEqual({
    height: 450,
    width: 800,
    x: 0,
    y: 0,
  });
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

test('fitCanvasToSources shrinks canvas to current source bounds', () => {
  const scene = createDefaultRecorderScene([
    monitor('main', { height: 900, width: 1440 }),
  ]);

  const fitted = fitCanvasToSources(scene);

  expect(fitted.layoutMode).toBe('custom');
  expect(fitted.canvasWidth).toBe(1440);
  expect(fitted.canvasHeight).toBe(900);
});

test('sceneSourceScale reports current scale against original dimensions', () => {
  const scene = createDefaultRecorderScene([
    monitor('ultrawide', { height: 1440, width: 5120 }),
  ]);

  expect(sceneSourceScale(sceneSource(scene))).toEqual({
    scale: 1,
    scaleX: 1,
    scaleY: 1,
    sourceHeight: 1440,
    sourceWidth: 5120,
  });
});

test('sceneSourceScale reports unknown when original dimensions are unknown', () => {
  const scene = createDefaultRecorderScene([monitor('unknown', {})]);

  expect(sceneSourceScale(sceneSource(scene))).toEqual({
    scale: null,
    scaleX: null,
    scaleY: null,
    sourceHeight: null,
    sourceWidth: null,
  });
});

const canvasTransformCases = [
  {
    expectedFill: { height: 540, width: 1920, x: 0, y: 270 },
    expectedFit: { height: 540, width: 1920, x: 0, y: 270 },
    locked: true,
    name: 'locked ultrawide source in 16:9 canvas',
    sourceHeight: 1440,
    sourceWidth: 5120,
  },
  {
    expectedFill: { height: 1080, width: 1920, x: 0, y: 0 },
    expectedFit: { height: 540, width: 1920, x: 0, y: 270 },
    locked: false,
    name: 'unlocked ultrawide source in 16:9 canvas',
    sourceHeight: 1440,
    sourceWidth: 5120,
  },
  {
    expectedFill: { height: 1080, width: 607.5, x: 656.25, y: 0 },
    expectedFit: { height: 1080, width: 607.5, x: 656.25, y: 0 },
    locked: true,
    name: 'locked portrait source in 16:9 canvas',
    sourceHeight: 1920,
    sourceWidth: 1080,
  },
  {
    expectedFill: { height: 1080, width: 1920, x: 0, y: 0 },
    expectedFit: { height: 1080, width: 607.5, x: 656.25, y: 0 },
    locked: false,
    name: 'unlocked portrait source in 16:9 canvas',
    sourceHeight: 1920,
    sourceWidth: 1080,
  },
  {
    expectedFill: { height: 1080, width: 1080, x: 420, y: 0 },
    expectedFit: { height: 1080, width: 1080, x: 420, y: 0 },
    locked: true,
    name: 'locked square source in 16:9 canvas',
    sourceHeight: 1000,
    sourceWidth: 1000,
  },
] as const;

test.each(canvasTransformCases)(
  'fit and fill transforms $name',
  ({ expectedFill, expectedFit, locked, sourceHeight, sourceWidth }) => {
    const scene = sceneWithSingleSource({
      canvasHeight: 1080,
      canvasWidth: 1920,
      locked,
      rect: {
        height: sourceHeight / 2,
        width: sourceWidth / 2,
        x: 0,
        y: 0,
      },
      sourceHeight,
      sourceWidth,
    });
    const sourceId = sceneSourceId(scene);

    expect(fitSceneSourceToCanvas(scene, sourceId).sources[0]?.rect).toEqual(
      expectedFit,
    );
    expect(fillSceneSourceToCanvas(scene, sourceId).sources[0]?.rect).toEqual(
      expectedFill,
    );
  },
);

test('locked fill uses the current rect aspect when original dimensions are unknown', () => {
  const scene = sceneWithSingleSource({
    canvasHeight: 1080,
    canvasWidth: 1920,
    locked: true,
    rect: {
      height: 500,
      width: 500,
      x: 120,
      y: 90,
    },
    sourceHeight: null,
    sourceWidth: null,
  });

  expect(
    fillSceneSourceToCanvas(scene, sceneSourceId(scene)).sources[0]?.rect,
  ).toEqual({
    height: 1080,
    width: 1080,
    x: 420,
    y: 0,
  });
});

test.each([
  {
    canvasHeight: 2880,
    canvasWidth: 5120,
    expectedRect: {
      height: 1440,
      width: 5120,
      x: 0,
      y: 720,
    },
    expectedScale: 1,
    name: 'native-width auto canvas',
  },
  {
    canvasHeight: 2160,
    canvasWidth: 3840,
    expectedRect: {
      height: 1080,
      width: 3840,
      x: 0,
      y: 540,
    },
    expectedScale: 0.75,
    name: '4k 16:9 canvas',
  },
  {
    canvasHeight: 1080,
    canvasWidth: 1920,
    expectedRect: {
      height: 540,
      width: 1920,
      x: 0,
      y: 270,
    },
    expectedScale: 0.375,
    name: '1080p 16:9 canvas',
  },
] as const)(
  'locked ultrawide fit and fill sequences contain inside $name',
  ({ canvasHeight, canvasWidth, expectedRect, expectedScale }) => {
    const scene = sceneWithSingleSource({
      canvasHeight,
      canvasWidth,
      locked: true,
      rect: {
        height: 1440,
        width: 5120,
        x: 0,
        y: 0,
      },
      sourceHeight: 1440,
      sourceWidth: 5120,
    });
    const sourceId = sceneSourceId(scene);

    const filled = fillSceneSourceToCanvas(scene, sourceId);
    const fitted = fitSceneSourceToCanvas(scene, sourceId);
    const fillThenFit = fitSceneSourceToCanvas(filled, sourceId);
    const fitThenFill = fillSceneSourceToCanvas(fitted, sourceId);
    const repeatedFill = fillSceneSourceToCanvas(filled, sourceId);
    const repeatedFit = fitSceneSourceToCanvas(fitted, sourceId);
    const fillCenterFit = fitSceneSourceToCanvas(
      centerSceneSource(filled, sourceId),
      sourceId,
    );

    for (const transformed of [
      filled,
      fitted,
      fillThenFit,
      fitThenFill,
      repeatedFill,
      repeatedFit,
      fillCenterFit,
    ]) {
      const source = sceneSource(transformed);
      expect(source.rect).toEqual(expectedRect);
      expect(source.rect.x).toBeGreaterThanOrEqual(0);
      expect(source.rect.y).toBeGreaterThanOrEqual(0);
      expect(source.rect.x + source.rect.width).toBeLessThanOrEqual(
        canvasWidth,
      );
      expect(source.rect.y + source.rect.height).toBeLessThanOrEqual(
        canvasHeight,
      );
      expect(sceneSourceScale(source).scale).toBe(expectedScale);
      expect(source.rect.width / source.rect.height).toBeCloseTo(5120 / 1440);
    }
  },
);

test('center transform selected source without resizing', () => {
  const scene = sceneWithSingleSource({
    canvasHeight: 1080,
    canvasWidth: 1920,
    locked: true,
    rect: {
      height: 360,
      width: 640,
      x: 0,
      y: 0,
    },
    sourceHeight: 720,
    sourceWidth: 1280,
  });

  expect(
    centerSceneSource(scene, sceneSourceId(scene)).sources[0]?.rect,
  ).toEqual({
    height: 360,
    width: 640,
    x: 640,
    y: 360,
  });
});

test('fillSceneSourceAvailableSpace fits into the largest non-overlapping gap', () => {
  const scene = createDefaultRecorderScene([
    monitor('left', { height: 1080, width: 960 }),
    monitor('right', { height: 1080, width: 960 }),
  ]);
  const rightSourceId = sceneSourceId(scene, 1);
  const customScene = {
    ...scene,
    layoutMode: 'custom' as const,
    sources: [
      {
        ...sceneSource(scene, 0),
        rect: {
          height: 1080,
          width: 960,
          x: 0,
          y: 0,
        },
      },
      {
        ...sceneSource(scene, 1),
        rect: {
          height: 100,
          width: 100,
          x: 0,
          y: 0,
        },
      },
    ],
  };

  const filled = fillSceneSourceAvailableSpace(customScene, rightSourceId);

  expect(filled.sources[1]?.rect).toEqual({
    height: 1080,
    width: 960,
    x: 960,
    y: 0,
  });
});

test.each([
  {
    expected: {
      height: 270,
      width: 960,
      x: 960,
      y: 0,
    },
    locked: true,
    name: 'locked ultrawide source preserves aspect inside the available gap',
  },
  {
    expected: {
      height: 1080,
      width: 960,
      x: 960,
      y: 0,
    },
    locked: false,
    name: 'unlocked ultrawide source distorts to the available gap',
  },
] as const)('fillSceneSourceAvailableSpace $name', ({ expected, locked }) => {
  const sourceScene = createDefaultRecorderScene([
    monitor('blocker', { height: 1080, width: 960 }),
    monitor('target', { height: 1440, width: 5120 }),
  ]);
  const targetSourceId = sceneSourceId(sourceScene, 1);
  const scene = {
    ...sourceScene,
    canvasHeight: 1080,
    canvasWidth: 1920,
    layoutMode: 'custom' as const,
    sources: [
      {
        ...sceneSource(sourceScene, 0),
        rect: {
          height: 1080,
          width: 960,
          x: 0,
          y: 0,
        },
      },
      {
        ...sceneSource(sourceScene, 1),
        preserveAspectRatio: locked,
        rect: {
          height: 100,
          width: 100,
          x: 0,
          y: 0,
        },
      },
    ],
  };

  const filled = fillSceneSourceAvailableSpace(scene, targetSourceId);

  expect(filled.sources[1]?.rect).toEqual(expected);
});

function sceneSourceId(scene: RecorderScene, index = 0): string {
  return sceneSource(scene, index).id;
}

function sceneWithSingleSource({
  canvasHeight,
  canvasWidth,
  locked,
  rect,
  sourceHeight,
  sourceWidth,
}: {
  canvasHeight: number;
  canvasWidth: number;
  locked: boolean;
  rect: CanvasRect;
  sourceHeight: null | number;
  sourceWidth: null | number;
}): RecorderScene {
  const scene = createDefaultRecorderScene([
    monitor('main', {
      height: sourceHeight,
      width: sourceWidth,
    }),
  ]);

  return {
    ...scene,
    canvasHeight,
    canvasWidth,
    layoutMode: 'custom',
    sources: [
      {
        ...sceneSource(scene),
        preserveAspectRatio: locked,
        rect,
        sourceHeight,
        sourceWidth,
      },
    ],
  };
}

function sceneSource(scene: RecorderScene, index = 0) {
  const source = scene.sources[index];
  if (!source) {
    throw new Error(`Expected scene source at index ${index}.`);
  }

  return source;
}
