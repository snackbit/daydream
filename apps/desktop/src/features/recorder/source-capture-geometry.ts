import type {
  CaptureSource,
  RecorderPreviewFrame,
  RecorderScene,
  RecorderSceneSource,
} from '@daydream/shared';

export function sourceDimensions(
  source: RecorderSceneSource,
  available: readonly CaptureSource[],
  preview: RecorderPreviewFrame | undefined,
) {
  const native = available.find(
    (candidate) => candidate.id === source.sourceId,
  );
  return {
    width: preview?.sourceWidth ?? source.sourceWidth ?? native?.width ?? null,
    height:
      preview?.sourceHeight ?? source.sourceHeight ?? native?.height ?? null,
  };
}
export function croppedDimensions(source: RecorderSceneSource) {
  return {
    width: (source.sourceWidth ?? 0) - source.crop.left - source.crop.right,
    height: (source.sourceHeight ?? 0) - source.crop.top - source.crop.bottom,
  };
}
function even(value: number) {
  return Math.floor(value / 2) * 2;
}

export function reviewSourceSetup(
  scene: RecorderScene,
  available: readonly CaptureSource[],
  previews: Record<string, RecorderPreviewFrame>,
): { scene: RecorderScene | null; error: string | null } {
  const sources: RecorderSceneSource[] = [];
  for (const source of scene.sources) {
    const { width, height } = sourceDimensions(
      source,
      available,
      previews[source.id],
    );
    if (!width || !height)
      return {
        scene: null,
        error: `Read the size of ${source.displayName} before saving. Select the source or start its preview.`,
      };
    const cropWidth = width - source.crop.left - source.crop.right,
      cropHeight = height - source.crop.top - source.crop.bottom;
    if (
      Object.values(source.crop).some(
        (value) => !Number.isInteger(value) || value < 0,
      ) ||
      cropWidth < 2 ||
      cropHeight < 2
    )
      return {
        scene: null,
        error: `The crop for ${source.displayName} must stay inside the source.`,
      };
    const outputWidth = even(
        scene.independentVideo ? source.rect.width : cropWidth,
      ),
      outputHeight = even(
        scene.independentVideo ? source.rect.height : cropHeight,
      );
    if (
      outputWidth < 2 ||
      outputHeight < 2 ||
      outputWidth > cropWidth ||
      outputHeight > cropHeight
    )
      return {
        scene: null,
        error: `Choose an output size within the cropped source for ${source.displayName}.`,
      };
    sources.push({
      ...source,
      sourceWidth: width,
      sourceHeight: height,
      zIndex: sources.length,
      preserveAspectRatio: true,
      rect: { x: 0, y: 0, width: outputWidth, height: outputHeight },
    });
  }
  return {
    scene: {
      ...scene,
      independentVideo: true,
      layoutMode: 'custom',
      sources,
      canvasWidth: Math.max(2, ...sources.map((source) => source.rect.width)),
      canvasHeight: Math.max(2, ...sources.map((source) => source.rect.height)),
    },
    error: null,
  };
}
export function resizeSource(
  source: RecorderSceneSource,
  scale: number,
): RecorderSceneSource {
  const crop = croppedDimensions(source);
  return {
    ...source,
    rect: {
      x: 0,
      y: 0,
      width: even(crop.width * scale),
      height: even(crop.height * scale),
    },
  };
}
export function cropSource(
  source: RecorderSceneSource,
  rect: { x: number; y: number; width: number; height: number },
): RecorderSceneSource {
  const previous = croppedDimensions(source);
  const scale = previous.width > 0 ? source.rect.width / previous.width : 1;
  return resizeSource(
    {
      ...source,
      crop: {
        left: rect.x,
        top: rect.y,
        right: (source.sourceWidth ?? 0) - rect.x - rect.width,
        bottom: (source.sourceHeight ?? 0) - rect.y - rect.height,
      },
    },
    scale,
  );
}
