import type {
  CanvasRect,
  CaptureSource,
  CropRect,
  RecorderScene,
  RecorderSceneLayoutMode,
  RecorderSceneSource,
} from '@daydream/shared';

const FALLBACK_SOURCE_WIDTH = 1280;
const FALLBACK_SOURCE_HEIGHT = 720;
const MIN_SOURCE_WIDTH = 64;
const MIN_SOURCE_HEIGHT = 36;
export const DEFAULT_SCENE_CANVAS_WIDTH = 1920;
export const DEFAULT_SCENE_CANVAS_HEIGHT = 1080;
export const SCENE_CANVAS_PRESETS = [
  {
    height: 1080,
    width: 1920,
  },
  {
    height: 1440,
    width: 2560,
  },
  {
    height: 2160,
    width: 3840,
  },
] as const;

export type ResizeHandle = 'e' | 'n' | 'ne' | 'nw' | 's' | 'se' | 'sw' | 'w';
export type SceneLayoutMode = Exclude<RecorderSceneLayoutMode, 'custom'>;
export type SceneSourceRectResizeDimension = 'auto' | 'height' | 'width';

export interface SceneCanvasSize {
  height: number;
  width: number;
}

export interface SceneSourceDimensions {
  height: number;
  source: 'fallback' | 'logical' | 'physical';
  width: number;
}

export interface SceneSourceScale {
  scale: number | null;
  scaleX: number | null;
  scaleY: number | null;
  sourceHeight: number | null;
  sourceWidth: number | null;
}

export interface DuplicateCaptureSourceCandidate {
  existingSource: RecorderSceneSource;
  source: CaptureSource;
}

export interface CaptureSourceMergeClassification {
  duplicateCandidates: DuplicateCaptureSourceCandidate[];
  exactDuplicates: CaptureSource[];
  newSources: CaptureSource[];
}

export type DuplicateCaptureSourceResolution =
  | 'add-anyway'
  | 'refresh-existing'
  | 'skip';

export interface MergeCaptureSourcesOptions {
  allowLikelyDuplicates?: boolean;
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
  const sceneSources = uniqueCaptureSources(sources).map((source, index) =>
    createSceneSource(source, index),
  );

  return applySceneLayout(
    {
      canvasHeight: 1,
      canvasWidth: 1,
      desktopAudioSourceId: null,
      id: newRecorderId(),
      layoutMode: 'auto-16-9',
      microphoneAudioSourceId: null,
      name: 'Recorder scene',
      sources: sceneSources,
    },
    'auto-16-9',
  );
}

export function classifyCaptureSourceMerge(
  scene: RecorderScene,
  sources: CaptureSource[],
): CaptureSourceMergeClassification {
  const existingSourceIds = new Set(
    scene.sources.map((source) => source.sourceId),
  );
  const existingSourcesBySignature = new Map(
    scene.sources.map((source) => [sceneSourceSignature(source), source]),
  );
  const seenSourceIds = new Set<string>();
  const seenSignatures = new Set<string>();
  const exactDuplicates: CaptureSource[] = [];
  const duplicateCandidates: DuplicateCaptureSourceCandidate[] = [];
  const newSources: CaptureSource[] = [];

  for (const source of sources) {
    const signature = captureSourceSignature(source);
    if (existingSourceIds.has(source.id) || seenSourceIds.has(source.id)) {
      exactDuplicates.push(source);
      continue;
    }

    const duplicateSource = existingSourcesBySignature.get(signature);
    if (duplicateSource) {
      duplicateCandidates.push({
        existingSource: duplicateSource,
        source,
      });
      seenSourceIds.add(source.id);
      seenSignatures.add(signature);
      continue;
    }

    if (seenSignatures.has(signature)) {
      exactDuplicates.push(source);
      seenSourceIds.add(source.id);
      continue;
    }

    newSources.push(source);
    seenSourceIds.add(source.id);
    seenSignatures.add(signature);
  }

  return {
    duplicateCandidates,
    exactDuplicates,
    newSources,
  };
}

export function resolveDuplicateCaptureSource(
  scene: RecorderScene,
  candidate: DuplicateCaptureSourceCandidate,
  resolution: DuplicateCaptureSourceResolution,
): RecorderScene {
  if (resolution === 'skip') {
    return scene;
  }

  if (resolution === 'refresh-existing') {
    const nativeDimensions = captureSourceNativeDimensions(candidate.source);
    return {
      ...scene,
      sources: scene.sources.map((source) =>
        source.id === candidate.existingSource.id
          ? {
              ...source,
              displayName: candidate.source.displayName,
              sourceId: candidate.source.id,
              sourceHeight: nativeDimensions?.height ?? null,
              sourceKind: candidate.source.kind,
              sourceWidth: nativeDimensions?.width ?? null,
            }
          : source,
      ),
    };
  }

  return mergeCaptureSourcesIntoScene(scene, [candidate.source], {
    allowLikelyDuplicates: true,
  });
}

export function mergeCaptureSourcesIntoScene(
  scene: RecorderScene,
  sources: CaptureSource[],
  options: MergeCaptureSourcesOptions = {},
): RecorderScene {
  const existingSourceIds = new Set(
    scene.sources.map((source) => source.sourceId),
  );
  const sourcesToMerge = options.allowLikelyDuplicates
    ? uniqueCaptureSources(sources)
    : classifyCaptureSourceMerge(scene, sources).newSources;

  if (sourcesToMerge.length === 0) {
    return scene;
  }

  const appendedScene = appendSourcesHorizontally(
    scene,
    sourcesToMerge,
    existingSourceIds,
  );

  if (scene.layoutMode === 'custom') {
    return expandCanvasToFitSources({
      ...appendedScene,
      layoutMode: 'custom',
    });
  }

  const layoutMode =
    scene.layoutMode === 'horizontal' || scene.layoutMode === 'vertical'
      ? scene.layoutMode
      : 'auto-16-9';
  const canvasSize = isPresetCanvasSize(scene)
    ? {
        height: scene.canvasHeight,
        width: scene.canvasWidth,
      }
    : undefined;

  return applySceneLayout(appendedScene, layoutMode, canvasSize);
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
      ? preserveNativeAspect(source, rect, handle, scene)
      : clampResizedRectToCanvas(source.rect, rect, handle, scene);

    return {
      ...source,
      crop: normalizeCrop(source.crop, nextRect),
      rect: nextRect,
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
  options: { resizeDimension?: SceneSourceRectResizeDimension } = {},
): RecorderScene {
  return updateSceneSource(scene, sourceId, (source) => {
    const requestedRect = {
      height: Math.max(MIN_SOURCE_HEIGHT, rect.height),
      width: Math.max(MIN_SOURCE_WIDTH, rect.width),
      x: rect.x,
      y: rect.y,
    };
    const aspectRect =
      source.preserveAspectRatio && options.resizeDimension
        ? rectWithNativeAspect(source, requestedRect, options.resizeDimension)
        : requestedRect;
    const clampedRect =
      source.preserveAspectRatio && options.resizeDimension
        ? clampAspectRectToCanvas(aspectRect, scene)
        : clampRectToCanvas(aspectRect, scene);

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
  if (!preserveAspectRatio) {
    return updateSceneSource(scene, sourceId, (source) => ({
      ...source,
      preserveAspectRatio,
    }));
  }

  const selectedSource = scene.sources.find((source) => source.id === sourceId);
  if (!selectedSource) {
    return scene;
  }

  const rect = fitSourceInAvailableSpace(scene, selectedSource);

  return updateSceneSource(scene, sourceId, (source) => ({
    ...source,
    crop: normalizeCrop(source.crop, rect),
    preserveAspectRatio,
    rect,
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

function createSceneSource(
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

function appendSourcesHorizontally(
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

function uniqueCaptureSources(sources: CaptureSource[]): CaptureSource[] {
  const seenSourceIds = new Set<string>();
  return sources.filter((source) => {
    if (seenSourceIds.has(source.id)) {
      return false;
    }

    seenSourceIds.add(source.id);
    return true;
  });
}

function layoutSourcesAuto(
  sources: RecorderSceneSource[],
  canvasSize: SceneCanvasSize,
  options: { allowUpscale?: boolean } = {},
): RecorderSceneSource[] {
  if (sources.length === 0) {
    return [];
  }

  const layout = bestAutoGrid(sources.length, canvasSize);
  return layoutSourcesInGrid(
    sources,
    layout.rows,
    layout.columns,
    canvasSize,
    options,
  );
}

function bestAutoGridForSources(sources: RecorderSceneSource[]): {
  columns: number;
  rows: number;
} {
  if (sources.length === 0) {
    return {
      columns: 1,
      rows: 1,
    };
  }

  const targetAspect = 16 / 9;
  const cellSize = naturalCellSize(sources);
  let best = {
    columns: 1,
    rows: sources.length,
    score: Number.POSITIVE_INFINITY,
  };

  for (let columns = 1; columns <= sources.length; columns += 1) {
    const rows = Math.ceil(sources.length / columns);
    const canvasWidth = cellSize.width * columns;
    const canvasHeight = cellSize.height * rows;
    const aspect = canvasWidth / canvasHeight;
    const emptyCells = rows * columns - sources.length;
    const score = Math.abs(aspect - targetAspect) + emptyCells * 0.01;
    if (score < best.score) {
      best = { columns, rows, score };
    }
  }

  return {
    columns: best.columns,
    rows: best.rows,
  };
}

function bestAutoGridForCanvas(
  sourceCount: number,
  canvasSize: SceneCanvasSize,
): { columns: number; rows: number } {
  const targetAspect = canvasSize.width / canvasSize.height;
  let best = {
    columns: 1,
    rows: sourceCount,
    score: Number.POSITIVE_INFINITY,
  };

  for (let columns = 1; columns <= sourceCount; columns += 1) {
    const rows = Math.ceil(sourceCount / columns);
    const aspect = columns / rows;
    const emptyCells = rows * columns - sourceCount;
    const score = Math.abs(aspect - targetAspect) + emptyCells * 0.01;
    if (score < best.score) {
      best = { columns, rows, score };
    }
  }

  return {
    columns: best.columns,
    rows: best.rows,
  };
}

function bestAutoGrid(
  sourceCount: number,
  canvasSize: SceneCanvasSize,
): { columns: number; rows: number } {
  return bestAutoGridForCanvas(sourceCount, canvasSize);
}

function layoutSourcesInGrid(
  sources: RecorderSceneSource[],
  rows: number,
  columns: number,
  canvasSize: SceneCanvasSize,
  options: { allowUpscale?: boolean } = {},
): RecorderSceneSource[] {
  if (sources.length === 0) {
    return [];
  }

  const safeColumns = Math.max(1, columns);
  const safeRows = Math.max(1, rows);
  const cellWidth = canvasSize.width / safeColumns;
  const cellHeight = canvasSize.height / safeRows;

  return sources.map((source, index) => {
    const column = index % safeColumns;
    const row = Math.floor(index / safeColumns);
    const rect = fitRectInCell(
      sourceNaturalRect(source),
      {
        height: cellHeight,
        width: cellWidth,
        x: column * cellWidth,
        y: row * cellHeight,
      },
      options,
    );

    return {
      ...source,
      crop: normalizeCrop(source.crop, rect),
      rect,
      zIndex: index,
    };
  });
}

function naturalCanvasSizeForLayout(
  sources: RecorderSceneSource[],
  mode: SceneLayoutMode,
): SceneCanvasSize {
  if (sources.length === 0) {
    return {
      height: DEFAULT_SCENE_CANVAS_HEIGHT,
      width: DEFAULT_SCENE_CANVAS_WIDTH,
    };
  }

  const cellSize = naturalCellSize(sources);
  const layout =
    mode === 'horizontal'
      ? {
          columns: sources.length,
          rows: 1,
        }
      : mode === 'vertical'
        ? {
            columns: 1,
            rows: sources.length,
          }
        : bestAutoGridForSources(sources);

  return {
    height: roundPixel(cellSize.height * layout.rows),
    width: roundPixel(cellSize.width * layout.columns),
  };
}

function naturalCellSize(sources: RecorderSceneSource[]): SceneCanvasSize {
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

function captureSourceNativeDimensions(
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

function sceneSourceNativeDimensions(
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

function sourceNaturalRect(source: RecorderSceneSource): CanvasRect {
  const dimensions = sceneSourceNativeDimensions(source);
  return {
    height: dimensions?.height ?? source.rect.height,
    width: dimensions?.width ?? source.rect.width,
    x: source.rect.x,
    y: source.rect.y,
  };
}

function sourceAspect(source: RecorderSceneSource): number | null {
  const dimensions = sceneSourceNativeDimensions(source);
  const aspect = dimensions
    ? dimensions.width / dimensions.height
    : source.rect.width / source.rect.height;

  return Number.isFinite(aspect) && aspect > 0 ? aspect : null;
}

function rectWithNativeAspect(
  source: RecorderSceneSource,
  rect: CanvasRect,
  resizeDimension: SceneSourceRectResizeDimension,
): CanvasRect {
  const aspect = sourceAspect(source);
  if (!aspect) {
    return rect;
  }

  const size =
    resizeDimension === 'width'
      ? normalizeAspectSize(rect.width, rect.width / aspect, aspect)
      : resizeDimension === 'height'
        ? normalizeAspectSize(rect.height * aspect, rect.height, aspect)
        : normalizeAspectSize(rect.width, rect.height, aspect);

  return {
    height: size.height,
    width: size.width,
    x: rect.x,
    y: rect.y,
  };
}

function isPresetCanvasSize(scene: RecorderScene): boolean {
  return SCENE_CANVAS_PRESETS.some(
    (preset) =>
      preset.width === scene.canvasWidth &&
      preset.height === scene.canvasHeight,
  );
}

function fitRectInCell(
  rect: CanvasRect,
  cell: CanvasRect,
  options: { allowUpscale?: boolean } = {},
): CanvasRect {
  const sourceAspect = rect.width / rect.height;
  if (!Number.isFinite(sourceAspect) || sourceAspect <= 0) {
    return {
      height: roundPixel(cell.height),
      width: roundPixel(cell.width),
      x: roundPixel(cell.x),
      y: roundPixel(cell.y),
    };
  }

  const scale = Math.min(
    cell.width / rect.width,
    cell.height / rect.height,
    options.allowUpscale ? Number.POSITIVE_INFINITY : 1,
  );
  const width = rect.width * scale;
  const height = rect.height * scale;

  return {
    height: roundPixel(height),
    width: roundPixel(width),
    x: roundPixel(cell.x + (cell.width - width) / 2),
    y: roundPixel(cell.y + (cell.height - height) / 2),
  };
}

function fitSourceInCell(
  source: RecorderSceneSource,
  cell: CanvasRect,
  options: { allowUpscale?: boolean } = {},
): CanvasRect {
  return fitRectInCell(sourceNaturalRect(source), cell, options);
}

function fitSourceInCellAround(
  source: RecorderSceneSource,
  cell: CanvasRect,
  options: { allowUpscale?: boolean; center?: Point } = {},
): CanvasRect {
  const rect = fitSourceInCell(source, cell, options);
  if (!options.center) {
    return rect;
  }

  return {
    ...rect,
    x: roundPixel(
      clamp(
        options.center.x - rect.width / 2,
        cell.x,
        cell.x + cell.width - rect.width,
      ),
    ),
    y: roundPixel(
      clamp(
        options.center.y - rect.height / 2,
        cell.y,
        cell.y + cell.height - rect.height,
      ),
    ),
  };
}

function fillRectInCell(cell: CanvasRect): CanvasRect {
  return {
    height: roundPixel(cell.height),
    width: roundPixel(cell.width),
    x: roundPixel(cell.x),
    y: roundPixel(cell.y),
  };
}

function transformSceneSource(
  scene: RecorderScene,
  sourceId: string,
  transform: (source: RecorderSceneSource) => CanvasRect,
  options: { clampToCanvas?: boolean } = {},
): RecorderScene {
  return updateSceneSource(scene, sourceId, (source) => {
    const rect =
      options.clampToCanvas === false
        ? normalizeRect(transform(source))
        : clampRectToCanvas(transform(source), scene);
    return {
      ...source,
      crop: normalizeCrop(source.crop, rect),
      rect,
    };
  });
}

function normalizeRect(rect: CanvasRect): CanvasRect {
  return {
    height: Math.max(MIN_SOURCE_HEIGHT, rect.height),
    width: Math.max(MIN_SOURCE_WIDTH, rect.width),
    x: rect.x,
    y: rect.y,
  };
}

function canvasRect(scene: RecorderScene): CanvasRect {
  return {
    height: scene.canvasHeight,
    width: scene.canvasWidth,
    x: 0,
    y: 0,
  };
}

function availableSpaceCandidates(
  scene: RecorderScene,
  sourceId: string,
): CanvasRect[] {
  const blockers = scene.sources
    .filter((source) => source.id !== sourceId)
    .map((source) => source.rect);
  const xCuts = uniqueSortedNumbers([
    0,
    scene.canvasWidth,
    ...blockers.flatMap((rect) => [rect.x, rect.x + rect.width]),
  ]);
  const yCuts = uniqueSortedNumbers([
    0,
    scene.canvasHeight,
    ...blockers.flatMap((rect) => [rect.y, rect.y + rect.height]),
  ]);
  const candidates: CanvasRect[] = [];

  for (let leftIndex = 0; leftIndex < xCuts.length - 1; leftIndex += 1) {
    for (
      let rightIndex = leftIndex + 1;
      rightIndex < xCuts.length;
      rightIndex += 1
    ) {
      const left = xCuts[leftIndex];
      const right = xCuts[rightIndex];
      if (left === undefined || right === undefined) {
        continue;
      }

      for (let topIndex = 0; topIndex < yCuts.length - 1; topIndex += 1) {
        for (
          let bottomIndex = topIndex + 1;
          bottomIndex < yCuts.length;
          bottomIndex += 1
        ) {
          const top = yCuts[topIndex];
          const bottom = yCuts[bottomIndex];
          if (top === undefined || bottom === undefined) {
            continue;
          }

          const candidate = {
            height: bottom - top,
            width: right - left,
            x: left,
            y: top,
          };
          if (
            candidate.width <= 0 ||
            candidate.height <= 0 ||
            blockers.some((rect) => rectsOverlap(candidate, rect))
          ) {
            continue;
          }
          candidates.push(candidate);
        }
      }
    }
  }

  return candidates;
}

function fitSourceInAvailableSpace(
  scene: RecorderScene,
  source: RecorderSceneSource,
): CanvasRect {
  const center = rectCenter(source.rect);
  const best = availableSpaceCandidates(scene, source.id)
    .map((candidate) => {
      const rect = fitSourceInCellAround(source, candidate, {
        allowUpscale: true,
        center,
      });
      return {
        area: rect.width * rect.height,
        distance: squaredDistance(center, rectCenter(rect)),
        rect,
      };
    })
    .filter(
      ({ rect }) =>
        rect.width >= MIN_SOURCE_WIDTH &&
        rect.height >= MIN_SOURCE_HEIGHT &&
        scene.sources
          .filter((candidate) => candidate.id !== source.id)
          .every((candidate) => !rectsOverlap(rect, candidate.rect)),
    )
    .sort(
      (left, right) => right.area - left.area || left.distance - right.distance,
    )[0]?.rect;

  return (
    best ??
    fitSourceInCellAround(source, canvasRect(scene), {
      allowUpscale: true,
      center,
    })
  );
}

interface Point {
  x: number;
  y: number;
}

function rectCenter(rect: CanvasRect): Point {
  return {
    x: rect.x + rect.width / 2,
    y: rect.y + rect.height / 2,
  };
}

function squaredDistance(left: Point, right: Point): number {
  const x = left.x - right.x;
  const y = left.y - right.y;
  return x * x + y * y;
}

function rectsOverlap(left: CanvasRect, right: CanvasRect): boolean {
  return (
    left.x < right.x + right.width &&
    left.x + left.width > right.x &&
    left.y < right.y + right.height &&
    left.y + left.height > right.y
  );
}

function uniqueSortedNumbers(values: number[]): number[] {
  return [...new Set(values.map(roundPixel))]
    .filter((value) => Number.isFinite(value))
    .sort((left, right) => left - right);
}

function captureSourceSignature(source: CaptureSource): string {
  return [source.kind, normalizeSourceName(source.displayName)].join(':');
}

function sceneSourceSignature(source: RecorderSceneSource): string {
  return [source.sourceKind, normalizeSourceName(source.displayName)].join(':');
}

function normalizeSourceName(value: string): string {
  return value.trim().replace(/\s+/g, ' ').toLocaleLowerCase();
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

function preserveNativeAspect(
  source: RecorderSceneSource,
  resized: CanvasRect,
  handle: ResizeHandle,
  scene: RecorderScene,
): CanvasRect {
  const aspect = sourceAspect(source);
  if (!aspect) {
    return clampResizedRectToCanvas(source.rect, resized, handle, scene);
  }

  const widthScale = resized.width / source.rect.width;
  const heightScale = resized.height / source.rect.height;
  const scale = Math.max(
    aspectScale(handle, widthScale, heightScale),
    MIN_SOURCE_WIDTH / source.rect.width,
    MIN_SOURCE_HEIGHT / source.rect.height,
  );
  const isHorizontalOnly =
    (handle.includes('e') || handle.includes('w')) &&
    !handle.includes('n') &&
    !handle.includes('s');
  const isVerticalOnly =
    (handle.includes('n') || handle.includes('s')) &&
    !handle.includes('e') &&
    !handle.includes('w');
  const requestedWidth = isVerticalOnly
    ? Math.max(MIN_SOURCE_WIDTH, resized.height * aspect)
    : Math.max(MIN_SOURCE_WIDTH, source.rect.width * scale);
  const requestedHeight = isHorizontalOnly
    ? Math.max(MIN_SOURCE_HEIGHT, requestedWidth / aspect)
    : isVerticalOnly
      ? Math.max(MIN_SOURCE_HEIGHT, resized.height)
      : Math.max(MIN_SOURCE_HEIGHT, source.rect.height * scale);
  const normalized = normalizeAspectSize(
    requestedWidth,
    requestedHeight,
    aspect,
  );
  const max = maxResizeSize(source.rect, handle, scene);
  const maxScale = Math.min(
    max.width / normalized.width,
    max.height / normalized.height,
    1,
  );
  const width = roundPixel(normalized.width * maxScale);
  const height = roundPixel(normalized.height * maxScale);

  return rectFromAnchoredSize(source.rect, handle, width, height);
}

function normalizeAspectSize(
  width: number,
  height: number,
  aspect: number,
): SceneCanvasSize {
  const widthDrivenHeight = width / aspect;
  const heightDrivenWidth = height * aspect;
  const useWidth =
    Math.abs(widthDrivenHeight - height) <= Math.abs(heightDrivenWidth - width);
  const normalizedWidth = useWidth ? width : heightDrivenWidth;
  const normalizedHeight = useWidth ? widthDrivenHeight : height;
  const minScale = Math.max(
    MIN_SOURCE_WIDTH / normalizedWidth,
    MIN_SOURCE_HEIGHT / normalizedHeight,
    1,
  );

  return {
    height: roundPixel(normalizedHeight * minScale),
    width: roundPixel(normalizedWidth * minScale),
  };
}

function clampResizedRectToCanvas(
  original: CanvasRect,
  resized: CanvasRect,
  handle: ResizeHandle,
  scene: RecorderScene,
): CanvasRect {
  const max = maxResizeSize(original, handle, scene);
  const width = roundPixel(clamp(resized.width, MIN_SOURCE_WIDTH, max.width));
  const height = roundPixel(
    clamp(resized.height, MIN_SOURCE_HEIGHT, max.height),
  );

  return rectFromAnchoredSize(original, handle, width, height);
}

function maxResizeSize(
  original: CanvasRect,
  handle: ResizeHandle,
  scene: RecorderScene,
): SceneCanvasSize {
  const fixedRight = original.x + original.width;
  const fixedBottom = original.y + original.height;

  return {
    height: Math.max(
      MIN_SOURCE_HEIGHT,
      handle.includes('n') ? fixedBottom : scene.canvasHeight - original.y,
    ),
    width: Math.max(
      MIN_SOURCE_WIDTH,
      handle.includes('w') ? fixedRight : scene.canvasWidth - original.x,
    ),
  };
}

function rectFromAnchoredSize(
  original: CanvasRect,
  handle: ResizeHandle,
  width: number,
  height: number,
): CanvasRect {
  return {
    height,
    width,
    x: roundPixel(
      handle.includes('w') ? original.x + original.width - width : original.x,
    ),
    y: roundPixel(
      handle.includes('n') ? original.y + original.height - height : original.y,
    ),
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

function clampAspectRectToCanvas(
  rect: CanvasRect,
  scene: RecorderScene,
): CanvasRect {
  const x = roundPixel(
    clamp(rect.x, 0, Math.max(0, scene.canvasWidth - MIN_SOURCE_WIDTH)),
  );
  const y = roundPixel(
    clamp(rect.y, 0, Math.max(0, scene.canvasHeight - MIN_SOURCE_HEIGHT)),
  );
  const maxScale = Math.min(
    (scene.canvasWidth - x) / rect.width,
    (scene.canvasHeight - y) / rect.height,
    1,
  );
  const width = roundPixel(rect.width * maxScale);
  const height = roundPixel(rect.height * maxScale);

  return {
    height,
    width,
    x: clamp(x, 0, Math.max(0, scene.canvasWidth - width)),
    y: clamp(y, 0, Math.max(0, scene.canvasHeight - height)),
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

function positiveNumberOrNull(value: number | null | undefined): number | null {
  return typeof value === 'number' && Number.isFinite(value) && value > 0
    ? value
    : null;
}

function normalizedCanvasSize(size: SceneCanvasSize): SceneCanvasSize {
  return {
    height: positiveOrFallback(size.height, DEFAULT_SCENE_CANVAS_HEIGHT),
    width: positiveOrFallback(size.width, DEFAULT_SCENE_CANVAS_WIDTH),
  };
}

function positiveOrFallback(value: number, fallback: number): number {
  return isPositive(value) ? Math.round(value) : fallback;
}

function isPositive(value: number | null): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value > 0;
}

function roundPixel(value: number): number {
  return Math.round(value * 100) / 100;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
