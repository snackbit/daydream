import type {
  CaptureSource,
  RecorderScene,
  RecorderSceneSource,
} from '@daydream/shared';
import {
  DEFAULT_SCENE_CANVAS_HEIGHT,
  DEFAULT_SCENE_CANVAS_WIDTH,
  MIN_SOURCE_HEIGHT,
  MIN_SOURCE_WIDTH,
} from './constants';
import {
  availableSpaceCandidates,
  bestAutoGridForSources,
  canvasRect,
  captureSourceNativeDimensions,
  clampRectToCanvas,
  fillRectInCell,
  fitSourceInCell,
  fitSourceInCellAround,
  layoutSourcesAuto,
  layoutSourcesInGrid,
  maxPositive,
  naturalCanvasSizeForLayout,
  normalizeCrop,
  normalizedCanvasSize,
  positiveNumberOrNull,
  rectCenter,
  roundPixel,
  transformSceneSource,
} from './internal';
import type {
  SceneCanvasSize,
  SceneLayoutMode,
  SceneSourceScale,
} from './types';

export function applySceneLayout(
  scene: RecorderScene,
  mode: SceneLayoutMode,
  canvasSize?: SceneCanvasSize,
): RecorderScene {
  const isAutoCanvas = canvasSize === undefined;
  const size = normalizedCanvasSize(
    canvasSize ?? naturalCanvasSizeForLayout(scene.sources, mode),
  );
  const fitOptions = {
    allowUpscale: !isAutoCanvas,
  };
  const sources =
    mode === 'horizontal'
      ? layoutSourcesInGrid(
          scene.sources,
          1,
          scene.sources.length,
          size,
          fitOptions,
        )
      : mode === 'vertical'
        ? layoutSourcesInGrid(
            scene.sources,
            scene.sources.length,
            1,
            size,
            fitOptions,
          )
        : isAutoCanvas
          ? layoutSourcesInGrid(
              scene.sources,
              bestAutoGridForSources(scene.sources).rows,
              bestAutoGridForSources(scene.sources).columns,
              size,
              fitOptions,
            )
          : layoutSourcesAuto(scene.sources, size, fitOptions);

  return {
    ...scene,
    canvasHeight: size.height,
    canvasWidth: size.width,
    layoutMode: mode,
    sources,
  };
}

export function setSceneCanvasSize(
  scene: RecorderScene,
  width: number,
  height: number,
): RecorderScene {
  const size = normalizedCanvasSize({ height, width });
  if (scene.layoutMode !== 'custom') {
    return applySceneLayout(scene, scene.layoutMode, size);
  }

  const nextScene = {
    ...scene,
    canvasHeight: size.height,
    canvasWidth: size.width,
    layoutMode: 'custom' as const,
  };

  return {
    ...nextScene,
    sources: nextScene.sources.map((source) => {
      const rect = clampRectToCanvas(source.rect, nextScene);
      return {
        ...source,
        crop: normalizeCrop(source.crop, rect),
        rect,
      };
    }),
  };
}

export function autoCanvasSizeForScene(scene: RecorderScene): SceneCanvasSize {
  if (scene.layoutMode === 'custom') {
    return {
      height: scene.canvasHeight,
      width: scene.canvasWidth,
    };
  }

  return naturalCanvasSizeForLayout(scene.sources, scene.layoutMode);
}

export function isSceneUsingAutoCanvas(scene: RecorderScene): boolean {
  if (scene.layoutMode === 'custom') {
    return false;
  }

  const autoSize = autoCanvasSizeForScene(scene);
  return (
    scene.canvasWidth === autoSize.width &&
    scene.canvasHeight === autoSize.height
  );
}

export function fitSceneSourceToCanvas(
  scene: RecorderScene,
  sourceId: string,
): RecorderScene {
  return transformSceneSource(scene, sourceId, (source) =>
    fitSourceInCell(source, canvasRect(scene), { allowUpscale: true }),
  );
}

export function fillSceneSourceToCanvas(
  scene: RecorderScene,
  sourceId: string,
): RecorderScene {
  return transformSceneSource(scene, sourceId, (source) =>
    source.preserveAspectRatio
      ? fitSourceInCell(source, canvasRect(scene), { allowUpscale: true })
      : fillRectInCell(canvasRect(scene)),
  );
}

export function centerSceneSource(
  scene: RecorderScene,
  sourceId: string,
): RecorderScene {
  return transformSceneSource(scene, sourceId, (source) => ({
    ...source.rect,
    x: roundPixel((scene.canvasWidth - source.rect.width) / 2),
    y: roundPixel((scene.canvasHeight - source.rect.height) / 2),
  }));
}

export function fillSceneSourceAvailableSpace(
  scene: RecorderScene,
  sourceId: string,
): RecorderScene {
  const source = scene.sources.find((source) => source.id === sourceId);
  if (!source) {
    return scene;
  }

  const candidates = availableSpaceCandidates(scene, sourceId);
  const best = candidates
    .map((candidate) => ({
      cell: candidate,
      rect: source.preserveAspectRatio
        ? fitSourceInCellAround(source, candidate, {
            allowUpscale: true,
            center: rectCenter(source.rect),
          })
        : fillRectInCell(candidate),
    }))
    .filter(
      ({ rect }) =>
        rect.width >= MIN_SOURCE_WIDTH && rect.height >= MIN_SOURCE_HEIGHT,
    )
    .sort((left, right) => {
      const leftArea = left.rect.width * left.rect.height;
      const rightArea = right.rect.width * right.rect.height;
      return (
        rightArea - leftArea ||
        left.rect.y - right.rect.y ||
        left.rect.x - right.rect.x
      );
    })[0]?.rect;

  return transformSceneSource(
    scene,
    sourceId,
    () =>
      best ??
      (source.preserveAspectRatio
        ? fitSourceInCellAround(source, canvasRect(scene), {
            allowUpscale: true,
            center: rectCenter(source.rect),
          })
        : fillRectInCell(canvasRect(scene))),
  );
}

export function sceneSourceScale(
  source: RecorderSceneSource,
  fallbackSource?: CaptureSource | null,
): SceneSourceScale {
  const fallbackDimensions = fallbackSource
    ? captureSourceNativeDimensions(fallbackSource)
    : null;
  const sourceWidth = positiveNumberOrNull(
    source.sourceWidth ?? fallbackDimensions?.width ?? null,
  );
  const sourceHeight = positiveNumberOrNull(
    source.sourceHeight ?? fallbackDimensions?.height ?? null,
  );

  return {
    scale: sourceWidth ? source.rect.width / sourceWidth : null,
    scaleX: sourceWidth ? source.rect.width / sourceWidth : null,
    scaleY: sourceHeight ? source.rect.height / sourceHeight : null,
    sourceHeight,
    sourceWidth,
  };
}

export function fitCanvasToSources(scene: RecorderScene): RecorderScene {
  const canvasWidth =
    maxPositive(
      scene.sources.map((source) => source.rect.x + source.rect.width),
    ) ?? DEFAULT_SCENE_CANVAS_WIDTH;
  const canvasHeight =
    maxPositive(
      scene.sources.map((source) => source.rect.y + source.rect.height),
    ) ?? DEFAULT_SCENE_CANVAS_HEIGHT;

  return {
    ...scene,
    canvasHeight: Math.ceil(canvasHeight),
    canvasWidth: Math.ceil(canvasWidth),
    layoutMode: 'custom',
  };
}

export function expandCanvasToFitSources(scene: RecorderScene): RecorderScene {
  const canvasWidth =
    maxPositive(
      scene.sources.map((source) => source.rect.x + source.rect.width),
    ) ?? scene.canvasWidth;
  const canvasHeight =
    maxPositive(
      scene.sources.map((source) => source.rect.y + source.rect.height),
    ) ?? scene.canvasHeight;

  return {
    ...scene,
    canvasHeight: Math.ceil(Math.max(scene.canvasHeight, canvasHeight)),
    canvasWidth: Math.ceil(Math.max(scene.canvasWidth, canvasWidth)),
  };
}
