import type {
  CanvasRect,
  RecorderScene,
  RecorderSceneSource,
} from '@daydream/shared';
import {
  clamp,
  naturalCellSize,
  roundPixel,
  sourceNaturalRect,
} from './common';
import {
  DEFAULT_SCENE_CANVAS_HEIGHT,
  DEFAULT_SCENE_CANVAS_WIDTH,
  MIN_SOURCE_HEIGHT,
  MIN_SOURCE_WIDTH,
} from './constants';
import {
  clampRectToCanvas,
  normalizeCrop,
  normalizeRect,
  updateSceneSource,
} from './editing-geometry';
import type { SceneCanvasSize, SceneLayoutMode } from './types';

export function layoutSourcesAuto(
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

export function bestAutoGridForSources(sources: RecorderSceneSource[]): {
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

export function bestAutoGridForCanvas(
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

export function bestAutoGrid(
  sourceCount: number,
  canvasSize: SceneCanvasSize,
): { columns: number; rows: number } {
  return bestAutoGridForCanvas(sourceCount, canvasSize);
}

export function layoutSourcesInGrid(
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

export function naturalCanvasSizeForLayout(
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

export function fitRectInCell(
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

export function fitSourceInCell(
  source: RecorderSceneSource,
  cell: CanvasRect,
  options: { allowUpscale?: boolean } = {},
): CanvasRect {
  return fitRectInCell(sourceNaturalRect(source), cell, options);
}

export function fitSourceInCellAround(
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

export function fillRectInCell(cell: CanvasRect): CanvasRect {
  return {
    height: roundPixel(cell.height),
    width: roundPixel(cell.width),
    x: roundPixel(cell.x),
    y: roundPixel(cell.y),
  };
}

export function transformSceneSource(
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

export function canvasRect(scene: RecorderScene): CanvasRect {
  return {
    height: scene.canvasHeight,
    width: scene.canvasWidth,
    x: 0,
    y: 0,
  };
}

export function availableSpaceCandidates(
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

export function fitSourceInAvailableSpace(
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

export interface Point {
  x: number;
  y: number;
}

export function rectCenter(rect: CanvasRect): Point {
  return {
    x: rect.x + rect.width / 2,
    y: rect.y + rect.height / 2,
  };
}

export function squaredDistance(left: Point, right: Point): number {
  const x = left.x - right.x;
  const y = left.y - right.y;
  return x * x + y * y;
}

export function rectsOverlap(left: CanvasRect, right: CanvasRect): boolean {
  return (
    left.x < right.x + right.width &&
    left.x + left.width > right.x &&
    left.y < right.y + right.height &&
    left.y + left.height > right.y
  );
}

export function uniqueSortedNumbers(values: number[]): number[] {
  return [...new Set(values.map(roundPixel))]
    .filter((value) => Number.isFinite(value))
    .sort((left, right) => left - right);
}
