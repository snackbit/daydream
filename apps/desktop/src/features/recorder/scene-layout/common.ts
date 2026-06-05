import type {
  CanvasRect,
  CaptureSource,
  CropRect,
  RecorderScene,
  RecorderSceneSource,
} from '@daydream/shared';
import {
  DEFAULT_SCENE_CANVAS_HEIGHT,
  DEFAULT_SCENE_CANVAS_WIDTH,
  FALLBACK_SOURCE_HEIGHT,
  FALLBACK_SOURCE_WIDTH,
  SCENE_CANVAS_PRESETS,
} from './constants';
import type { SceneCanvasSize, SceneSourceDimensions } from './types';

export function sourceDimensions(source: CaptureSource): SceneSourceDimensions {
  if (isPositive(source.width) && isPositive(source.height)) {
    return {
      height: source.height,
      source: 'physical',
      width: source.width,
    };
  }

  if (isPositive(source.logicalWidth) && isPositive(source.logicalHeight)) {
    return {
      height: source.logicalHeight,
      source: 'logical',
      width: source.logicalWidth,
    };
  }

  return {
    height: FALLBACK_SOURCE_HEIGHT,
    source: 'fallback',
    width: FALLBACK_SOURCE_WIDTH,
  };
}
export function createSceneSource(
  source: CaptureSource,
  zIndex: number,
): RecorderSceneSource {
  const dimensions = sourceDimensions(source);
  const nativeDimensions = captureSourceNativeDimensions(source);

  return {
    crop: emptyCrop(),
    displayName: source.displayName,
    id: newRecorderId(),
    preserveAspectRatio: true,
    rect: {
      height: dimensions.height,
      width: dimensions.width,
      x: 0,
      y: 0,
    },
    sourceId: source.id,
    sourceHeight: nativeDimensions?.height ?? null,
    sourceKind: source.kind,
    sourceWidth: nativeDimensions?.width ?? null,
    zIndex,
  };
}

export function appendSourcesHorizontally(
  scene: RecorderScene,
  sources: CaptureSource[],
  existingSourceIds: Set<string>,
): RecorderScene {
  let nextX = nextHorizontalSourceX(scene);
  let nextZIndex =
    Math.max(-1, ...scene.sources.map((source) => source.zIndex)) + 1;
  const newSceneSources: RecorderSceneSource[] = [];

  for (const source of sources) {
    if (existingSourceIds.has(source.id)) {
      continue;
    }

    const sceneSource = createSceneSource(source, nextZIndex);
    newSceneSources.push({
      ...sceneSource,
      rect: {
        ...sceneSource.rect,
        x: nextX,
      },
    });

    nextX += sceneSource.rect.width;
    nextZIndex += 1;
    existingSourceIds.add(source.id);
  }

  return {
    ...scene,
    sources: [...scene.sources, ...newSceneSources],
  };
}

export function uniqueCaptureSources(
  sources: CaptureSource[],
): CaptureSource[] {
  const seenSourceIds = new Set<string>();
  return sources.filter((source) => {
    if (seenSourceIds.has(source.id)) {
      return false;
    }

    seenSourceIds.add(source.id);
    return true;
  });
}

export function naturalCellSize(
  sources: RecorderSceneSource[],
): SceneCanvasSize {
  return {
    height:
      maxPositive(
        sources.map((source) => source.sourceHeight ?? source.rect.height),
      ) ?? DEFAULT_SCENE_CANVAS_HEIGHT,
    width:
      maxPositive(
        sources.map((source) => source.sourceWidth ?? source.rect.width),
      ) ?? DEFAULT_SCENE_CANVAS_WIDTH,
  };
}

export function captureSourceNativeDimensions(
  source: CaptureSource,
): SceneCanvasSize | null {
  if (isPositive(source.width) && isPositive(source.height)) {
    return {
      height: source.height,
      width: source.width,
    };
  }

  if (isPositive(source.logicalWidth) && isPositive(source.logicalHeight)) {
    return {
      height: source.logicalHeight,
      width: source.logicalWidth,
    };
  }

  return null;
}

export function sceneSourceNativeDimensions(
  source: RecorderSceneSource,
): SceneCanvasSize | null {
  if (isPositive(source.sourceWidth) && isPositive(source.sourceHeight)) {
    return {
      height: source.sourceHeight,
      width: source.sourceWidth,
    };
  }

  return null;
}

export function sourceNaturalRect(source: RecorderSceneSource): CanvasRect {
  const dimensions = sceneSourceNativeDimensions(source);
  return {
    height: dimensions?.height ?? source.rect.height,
    width: dimensions?.width ?? source.rect.width,
    x: source.rect.x,
    y: source.rect.y,
  };
}

export function sourceAspect(source: RecorderSceneSource): number | null {
  const dimensions = sceneSourceNativeDimensions(source);
  const aspect = dimensions
    ? dimensions.width / dimensions.height
    : source.rect.width / source.rect.height;

  return Number.isFinite(aspect) && aspect > 0 ? aspect : null;
}

export function isPresetCanvasSize(scene: RecorderScene): boolean {
  return SCENE_CANVAS_PRESETS.some(
    (preset) =>
      preset.width === scene.canvasWidth &&
      preset.height === scene.canvasHeight,
  );
}

export function captureSourceSignature(source: CaptureSource): string {
  return [source.kind, normalizeSourceName(source.displayName)].join(':');
}

export function sceneSourceSignature(source: RecorderSceneSource): string {
  return [source.sourceKind, normalizeSourceName(source.displayName)].join(':');
}

export function normalizeSourceName(value: string): string {
  return value.trim().replace(/\s+/g, ' ').toLocaleLowerCase();
}

export function nextHorizontalSourceX(scene: RecorderScene): number {
  return (
    maxPositive(
      scene.sources.map((source) => source.rect.x + source.rect.width),
    ) ?? 0
  );
}

export function normalizeZOrderIfNeeded(
  sources: RecorderSceneSource[],
): RecorderSceneSource[] {
  const sortedSources = [...sources].sort(
    (left, right) =>
      left.zIndex - right.zIndex || left.id.localeCompare(right.id),
  );
  const needsNormalization = sortedSources.some(
    (source, index) => source.zIndex !== index,
  );

  if (!needsNormalization) {
    return sources;
  }

  const normalizedZIndexById = new Map(
    sortedSources.map((source, index) => [source.id, index]),
  );

  return sources.map((source) => ({
    ...source,
    zIndex: normalizedZIndexById.get(source.id) ?? source.zIndex,
  }));
}

export function newRecorderId(): string {
  return crypto.randomUUID();
}

export function emptyCrop(): CropRect {
  return {
    bottom: 0,
    left: 0,
    right: 0,
    top: 0,
  };
}

export function maxPositive(values: number[]): number | null {
  const positives = values.filter(isPositive);
  return positives.length > 0 ? Math.max(...positives) : null;
}

export function positiveNumberOrNull(
  value: number | null | undefined,
): number | null {
  return typeof value === 'number' && Number.isFinite(value) && value > 0
    ? value
    : null;
}

export function normalizedCanvasSize(size: SceneCanvasSize): SceneCanvasSize {
  return {
    height: positiveOrFallback(size.height, DEFAULT_SCENE_CANVAS_HEIGHT),
    width: positiveOrFallback(size.width, DEFAULT_SCENE_CANVAS_WIDTH),
  };
}

export function positiveOrFallback(value: number, fallback: number): number {
  return isPositive(value) ? Math.round(value) : fallback;
}

export function isPositive(value: number | null): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value > 0;
}

export function roundPixel(value: number): number {
  return Math.round(value * 100) / 100;
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
