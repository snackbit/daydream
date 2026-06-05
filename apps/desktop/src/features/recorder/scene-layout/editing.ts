import type {
  CanvasRect,
  CropRect,
  RecorderScene,
  RecorderSceneSource,
} from '@daydream/shared';
import { MIN_SOURCE_HEIGHT, MIN_SOURCE_WIDTH } from './constants';
import {
  clampAspectRectToCanvas,
  clampRectToCanvas,
  clampResizedRectToCanvas,
  fitSourceInAvailableSpace,
  normalizeCrop,
  preserveNativeAspect,
  rectWithNativeAspect,
  resizeRect,
  updateSceneSource,
} from './internal';
import type { ResizeHandle, SceneSourceRectResizeDimension } from './types';

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
