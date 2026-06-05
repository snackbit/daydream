import type {
  CanvasRect,
  CaptureSource,
  CropRect,
  RecorderScene,
  RecorderSceneSource,
} from '@daydream/shared';

const FALLBACK_SOURCE_WIDTH = 1280;
const FALLBACK_SOURCE_HEIGHT = 720;
const MIN_SOURCE_WIDTH = 64;
const MIN_SOURCE_HEIGHT = 36;

export type ResizeHandle = 'e' | 'n' | 'ne' | 'nw' | 's' | 'se' | 'sw' | 'w';

export interface SceneSourceDimensions {
  height: number;
  source: 'fallback' | 'logical' | 'physical';
  width: number;
}

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

export function createDefaultRecorderScene(
  sources: CaptureSource[],
): RecorderScene {
  let nextX = 0;
  const sceneSources = sources.map((source, index) => {
    const dimensions = sourceDimensions(source);
    const sceneSource: RecorderSceneSource = {
      crop: emptyCrop(),
      displayName: source.displayName,
      id: newRecorderId(),
      preserveAspectRatio: true,
      rect: {
        height: dimensions.height,
        width: dimensions.width,
        x: nextX,
        y: 0,
      },
      sourceId: source.id,
      sourceKind: source.kind,
      zIndex: index,
    };

    nextX += dimensions.width;
    return sceneSource;
  });

  return {
    canvasHeight:
      maxPositive(sceneSources.map((source) => source.rect.height)) ??
      FALLBACK_SOURCE_HEIGHT,
    canvasWidth:
      maxPositive([nextX]) ??
      (sources.length > 0 ? nextX : FALLBACK_SOURCE_WIDTH),
    id: newRecorderId(),
    layoutMode: 'horizontal',
    name: 'Recorder scene',
    sources: sceneSources,
  };
}

export function mergeCaptureSourcesIntoScene(
  scene: RecorderScene,
  sources: CaptureSource[],
): RecorderScene {
  const existingSourceIds = new Set(
    scene.sources.map((source) => source.sourceId),
  );
  let nextX = nextHorizontalSourceX(scene);
  let nextZIndex =
    Math.max(-1, ...scene.sources.map((source) => source.zIndex)) + 1;
  const newSceneSources: RecorderSceneSource[] = [];

  for (const source of sources) {
    if (existingSourceIds.has(source.id)) {
      continue;
    }

    const dimensions = sourceDimensions(source);
    newSceneSources.push({
      crop: emptyCrop(),
      displayName: source.displayName,
      id: newRecorderId(),
      preserveAspectRatio: true,
      rect: {
        height: dimensions.height,
        width: dimensions.width,
        x: nextX,
        y: 0,
      },
      sourceId: source.id,
      sourceKind: source.kind,
      zIndex: nextZIndex,
    });

    nextX += dimensions.width;
    nextZIndex += 1;
    existingSourceIds.add(source.id);
  }

  if (newSceneSources.length === 0) {
    return scene;
  }

  return expandCanvasToFitSources({
    ...scene,
    layoutMode: 'custom',
    sources: [...scene.sources, ...newSceneSources],
  });
}

export function removeSceneSource(
  scene: RecorderScene,
  sceneSourceId: string,
): RecorderScene {
  if (!scene.sources.some((source) => source.id === sceneSourceId)) {
    return scene;
  }

  const sources = normalizeZOrderIfNeeded(
    scene.sources.filter((source) => source.id !== sceneSourceId),
  );

  return {
    ...scene,
    layoutMode: 'custom',
    sources,
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

export function moveSceneSource(
  scene: RecorderScene,
  sourceId: string,
  deltaX: number,
  deltaY: number,
): RecorderScene {
  return updateSceneSource(scene, sourceId, (source) => ({
    ...source,
    rect: clampRectToCanvas(
      {
        ...source.rect,
        x: source.rect.x + deltaX,
        y: source.rect.y + deltaY,
      },
      scene,
    ),
  }));
}

export function resizeSceneSource(
  scene: RecorderScene,
  sourceId: string,
  handle: ResizeHandle,
  deltaX: number,
  deltaY: number,
): RecorderScene {
  return updateSceneSource(scene, sourceId, (source) => {
    const rect = resizeRect(source.rect, handle, deltaX, deltaY);
    const nextRect = source.preserveAspectRatio
      ? preserveAspect(source.rect, rect, handle)
      : rect;
    const clampedRect = clampRectToCanvas(nextRect, scene);

    return {
      ...source,
      crop: normalizeCrop(source.crop, clampedRect),
      rect: clampedRect,
    };
  });
}

export function setSceneSourceCrop(
  scene: RecorderScene,
  sourceId: string,
  crop: CropRect,
): RecorderScene {
  return updateSceneSource(scene, sourceId, (source) => ({
    ...source,
    crop: normalizeCrop(crop, source.rect),
  }));
}

export function setSceneSourceRect(
  scene: RecorderScene,
  sourceId: string,
  rect: CanvasRect,
): RecorderScene {
  return updateSceneSource(scene, sourceId, (source) => {
    const clampedRect = clampRectToCanvas(
      {
        height: Math.max(MIN_SOURCE_HEIGHT, rect.height),
        width: Math.max(MIN_SOURCE_WIDTH, rect.width),
        x: rect.x,
        y: rect.y,
      },
      scene,
    );

    return {
      ...source,
      crop: normalizeCrop(source.crop, clampedRect),
      rect: clampedRect,
    };
  });
}

export function setSceneSourceAspectLock(
  scene: RecorderScene,
  sourceId: string,
  preserveAspectRatio: boolean,
): RecorderScene {
  return updateSceneSource(scene, sourceId, (source) => ({
    ...source,
    preserveAspectRatio,
  }));
}

export function raiseSceneSource(
  scene: RecorderScene,
  sourceId: string,
): RecorderScene {
  const maxZ = Math.max(0, ...scene.sources.map((source) => source.zIndex));
  return updateSceneSource(scene, sourceId, (source) => ({
    ...source,
    zIndex: maxZ + 1,
  }));
}

export function lowerSceneSource(
  scene: RecorderScene,
  sourceId: string,
): RecorderScene {
  const minZ = Math.min(0, ...scene.sources.map((source) => source.zIndex));
  return updateSceneSource(scene, sourceId, (source) => ({
    ...source,
    zIndex: minZ - 1,
  }));
}

export function sortedSceneSources(
  scene: RecorderScene,
): RecorderSceneSource[] {
  return [...scene.sources].sort((left, right) => left.zIndex - right.zIndex);
}

export function formatPixels(value: number): string {
  return Math.round(value).toString();
}

function resizeRect(
  rect: CanvasRect,
  handle: ResizeHandle,
  deltaX: number,
  deltaY: number,
): CanvasRect {
  const next = { ...rect };

  if (handle.includes('e')) {
    next.width = rect.width + deltaX;
  }

  if (handle.includes('s')) {
    next.height = rect.height + deltaY;
  }

  if (handle.includes('w')) {
    next.x = rect.x + deltaX;
    next.width = rect.width - deltaX;
  }

  if (handle.includes('n')) {
    next.y = rect.y + deltaY;
    next.height = rect.height - deltaY;
  }

  const width = Math.max(MIN_SOURCE_WIDTH, next.width);
  const height = Math.max(MIN_SOURCE_HEIGHT, next.height);

  return {
    height,
    width,
    x: handle.includes('w') ? rect.x + rect.width - width : next.x,
    y: handle.includes('n') ? rect.y + rect.height - height : next.y,
  };
}

function preserveAspect(
  original: CanvasRect,
  resized: CanvasRect,
  handle: ResizeHandle,
): CanvasRect {
  const aspect = original.width / original.height;
  if (!Number.isFinite(aspect) || aspect <= 0) {
    return resized;
  }

  const widthScale = resized.width / original.width;
  const heightScale = resized.height / original.height;
  const scale = aspectScale(handle, widthScale, heightScale);
  const width = Math.max(MIN_SOURCE_WIDTH, original.width * scale);
  const height = Math.max(MIN_SOURCE_HEIGHT, width / aspect);

  return {
    height,
    width,
    x: handle.includes('w') ? original.x + original.width - width : original.x,
    y: handle.includes('n')
      ? original.y + original.height - height
      : original.y,
  };
}

function clampRectToCanvas(rect: CanvasRect, scene: RecorderScene): CanvasRect {
  const width = Math.min(
    Math.max(MIN_SOURCE_WIDTH, rect.width),
    scene.canvasWidth,
  );
  const height = Math.min(
    Math.max(MIN_SOURCE_HEIGHT, rect.height),
    scene.canvasHeight,
  );

  return {
    height,
    width,
    x: clamp(rect.x, 0, Math.max(0, scene.canvasWidth - width)),
    y: clamp(rect.y, 0, Math.max(0, scene.canvasHeight - height)),
  };
}

function aspectScale(
  handle: ResizeHandle,
  widthScale: number,
  heightScale: number,
): number {
  if (handle.includes('e') || handle.includes('w')) {
    if (!handle.includes('n') && !handle.includes('s')) {
      return widthScale;
    }
  }

  if (handle.includes('n') || handle.includes('s')) {
    if (!handle.includes('e') && !handle.includes('w')) {
      return heightScale;
    }
  }

  return Math.abs(widthScale - 1) >= Math.abs(heightScale - 1)
    ? widthScale
    : heightScale;
}

function normalizeCrop(crop: CropRect, rect: CanvasRect): CropRect {
  const maxHorizontalCrop = Math.max(0, rect.width - 1);
  const left = clamp(crop.left, 0, maxHorizontalCrop);
  const right = clamp(crop.right, 0, Math.max(0, maxHorizontalCrop - left));
  const maxVerticalCrop = Math.max(0, rect.height - 1);
  const top = clamp(crop.top, 0, maxVerticalCrop);
  const bottom = clamp(crop.bottom, 0, Math.max(0, maxVerticalCrop - top));

  return { bottom, left, right, top };
}

function updateSceneSource(
  scene: RecorderScene,
  sourceId: string,
  updater: (source: RecorderSceneSource) => RecorderSceneSource,
): RecorderScene {
  return {
    ...scene,
    layoutMode: 'custom',
    sources: scene.sources.map((source) =>
      source.id === sourceId ? updater(source) : source,
    ),
  };
}

function nextHorizontalSourceX(scene: RecorderScene): number {
  return (
    maxPositive(
      scene.sources.map((source) => source.rect.x + source.rect.width),
    ) ?? 0
  );
}

function normalizeZOrderIfNeeded(
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

function newRecorderId(): string {
  return crypto.randomUUID();
}

function emptyCrop(): CropRect {
  return {
    bottom: 0,
    left: 0,
    right: 0,
    top: 0,
  };
}

function maxPositive(values: number[]): number | null {
  const positives = values.filter(isPositive);
  return positives.length > 0 ? Math.max(...positives) : null;
}

function isPositive(value: number | null): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value > 0;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
