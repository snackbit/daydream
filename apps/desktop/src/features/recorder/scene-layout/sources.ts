import type { CaptureSource, RecorderScene } from '@daydream/shared';
import {
  appendSourcesHorizontally,
  captureSourceNativeDimensions,
  captureSourceSignature,
  createSceneSource,
  isPresetCanvasSize,
  newRecorderId,
  normalizeZOrderIfNeeded,
  sceneSourceSignature,
  sourceDimensions,
  uniqueCaptureSources,
} from './internal';
import { applySceneLayout, expandCanvasToFitSources } from './layout';
import type {
  CaptureSourceMergeClassification,
  DuplicateCaptureSourceCandidate,
  DuplicateCaptureSourceResolution,
  MergeCaptureSourcesOptions,
} from './types';

export { sourceDimensions };

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
