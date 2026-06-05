import type {
  CanvasRect,
  CropRect,
  RecorderScene,
  RecorderSceneSource,
} from '@daydream/shared';
import { clamp, roundPixel, sourceAspect } from './common';
import { MIN_SOURCE_HEIGHT, MIN_SOURCE_WIDTH } from './constants';
import type {
  ResizeHandle,
  SceneCanvasSize,
  SceneSourceRectResizeDimension,
} from './types';

export function rectWithNativeAspect(
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

export function normalizeRect(rect: CanvasRect): CanvasRect {
  return {
    height: Math.max(MIN_SOURCE_HEIGHT, rect.height),
    width: Math.max(MIN_SOURCE_WIDTH, rect.width),
    x: rect.x,
    y: rect.y,
  };
}

export function resizeRect(
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

export function preserveNativeAspect(
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

export function normalizeAspectSize(
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

export function clampResizedRectToCanvas(
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

export function maxResizeSize(
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

export function rectFromAnchoredSize(
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

export function clampRectToCanvas(
  rect: CanvasRect,
  scene: RecorderScene,
): CanvasRect {
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

export function clampAspectRectToCanvas(
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

export function aspectScale(
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

export function normalizeCrop(crop: CropRect, rect: CanvasRect): CropRect {
  const maxHorizontalCrop = Math.max(0, rect.width - 1);
  const left = clamp(crop.left, 0, maxHorizontalCrop);
  const right = clamp(crop.right, 0, Math.max(0, maxHorizontalCrop - left));
  const maxVerticalCrop = Math.max(0, rect.height - 1);
  const top = clamp(crop.top, 0, maxVerticalCrop);
  const bottom = clamp(crop.bottom, 0, Math.max(0, maxVerticalCrop - top));

  return { bottom, left, right, top };
}

export function updateSceneSource(
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
