import type {
  CanvasRect,
  CaptureSource,
  CropRect,
  RecorderScene,
  RecorderSceneSource,
} from '@daydream/shared';
import {
  ArrowDown,
  ArrowUp,
  Crop,
  Crosshair,
  Expand,
  Maximize2,
  Minimize2,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldLabel,
} from '@/components/ui/field';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import {
  centerSceneSource,
  fillSceneSourceAvailableSpace,
  fillSceneSourceToCanvas,
  fitSceneSourceToCanvas,
  lowerSceneSource,
  raiseSceneSource,
  sceneSourceScale,
  setSceneSourceAspectLock,
  setSceneSourceCrop,
  setSceneSourceRect,
} from '@/features/recorder/scene-layout';
import type { SceneSourceRectResizeDimension } from '@/features/recorder/scene-layout';
import { PixelInputField } from './pixel-input-field';

export interface GeometryPanelProps {
  captureSources: CaptureSource[];
  onChange: (scene: RecorderScene) => void;
  scene: RecorderScene | null;
  source: RecorderSceneSource | null;
}

export function GeometryPanel({
  captureSources,
  onChange,
  scene,
  source,
}: GeometryPanelProps) {
  if (!scene || !source) {
    return (
      <ScrollArea className="h-full">
        <div className="p-4">
          <h2 className="text-sm font-medium">Geometry</h2>
          <Card className="mt-3" size="sm">
            <CardContent className="text-muted-foreground text-xs">
              No source selected.
            </CardContent>
          </Card>
        </div>
      </ScrollArea>
    );
  }

  const updateRect = (
    partial: Partial<CanvasRect>,
    resizeDimension?: SceneSourceRectResizeDimension,
  ) => {
    const rect = {
      ...source.rect,
      ...partial,
    };
    onChange(
      resizeDimension
        ? setSceneSourceRect(scene, source.id, rect, { resizeDimension })
        : setSceneSourceRect(scene, source.id, rect),
    );
  };
  const updateCrop = (partial: Partial<CropRect>) => {
    onChange(
      setSceneSourceCrop(scene, source.id, {
        ...source.crop,
        ...partial,
      }),
    );
  };
  const aspectInputId = `preserve-aspect-${source.id}`;
  const matchingCaptureSource =
    captureSources.find(
      (captureSource) => captureSource.id === source.sourceId,
    ) ?? null;
  const scale = sceneSourceScale(source, matchingCaptureSource);

  return (
    <ScrollArea className="h-full">
      <div className="p-4">
        <h2 className="text-sm font-medium">Geometry</h2>
        <Card className="mt-3" size="sm">
          <CardHeader className="border-b pb-3">
            <CardTitle>{source.displayName}</CardTitle>
            <div className="text-muted-foreground mt-1 font-mono text-xs">
              {source.sourceId}
            </div>
          </CardHeader>
          <CardContent className="grid gap-4 pt-3">
            <div className="text-muted-foreground grid gap-1 text-xs">
              {scale.sourceWidth && scale.sourceHeight ? (
                <>
                  <div className="flex items-center justify-between gap-2">
                    <span>Original</span>
                    <span className="font-mono">
                      {formatDimension(scale.sourceWidth)} x{' '}
                      {formatDimension(scale.sourceHeight)}
                    </span>
                  </div>
                  <div className="flex items-center justify-between gap-2">
                    <span>Scale</span>
                    <span className="font-mono">
                      {formatScaleLabel(scale, source.preserveAspectRatio)}
                    </span>
                  </div>
                </>
              ) : (
                <div className="flex items-center justify-between gap-2">
                  <span>Original</span>
                  <span className="font-mono">unknown</span>
                </div>
              )}
            </div>

            <div className="grid grid-cols-2 gap-2">
              <PixelInputField
                label="X"
                value={source.rect.x}
                onChange={(value) => updateRect({ x: value })}
              />
              <PixelInputField
                label="Y"
                value={source.rect.y}
                onChange={(value) => updateRect({ y: value })}
              />
              <PixelInputField
                label="W"
                value={source.rect.width}
                onChange={(value) => updateRect({ width: value }, 'width')}
              />
              <PixelInputField
                label="H"
                value={source.rect.height}
                onChange={(value) => updateRect({ height: value }, 'height')}
              />
            </div>

            <Field orientation="horizontal">
              <Checkbox
                checked={source.preserveAspectRatio}
                id={aspectInputId}
                onCheckedChange={(checked) =>
                  onChange(
                    setSceneSourceAspectLock(
                      scene,
                      source.id,
                      checked === true,
                    ),
                  )
                }
              />
              <FieldContent>
                <FieldLabel htmlFor={aspectInputId}>Aspect</FieldLabel>
                <FieldDescription>
                  Preserve ratio while resizing
                </FieldDescription>
              </FieldContent>
            </Field>

            <div className="grid grid-cols-2 gap-2">
              <Button
                className="gap-2"
                onClick={() =>
                  onChange(fitSceneSourceToCanvas(scene, source.id))
                }
                size="sm"
                variant="outline"
              >
                <Minimize2 className="size-3" />
                Fit
              </Button>
              <Button
                className="gap-2"
                onClick={() =>
                  onChange(fillSceneSourceToCanvas(scene, source.id))
                }
                size="sm"
                variant="outline"
              >
                <Maximize2 className="size-3" />
                Fill
              </Button>
              <Button
                className="gap-2"
                onClick={() => onChange(centerSceneSource(scene, source.id))}
                size="sm"
                variant="outline"
              >
                <Crosshair className="size-3" />
                Center
              </Button>
              <Button
                className="gap-2"
                onClick={() =>
                  onChange(fillSceneSourceAvailableSpace(scene, source.id))
                }
                size="sm"
                variant="outline"
              >
                <Expand className="size-3" />
                Fill empty
              </Button>
            </div>

            <Separator />

            <div>
              <div className="mb-2 flex items-center gap-2 text-xs font-medium">
                <Crop className="size-3" />
                Crop
              </div>
              <div className="grid grid-cols-2 gap-2">
                <PixelInputField
                  label="L"
                  value={source.crop.left}
                  onChange={(value) => updateCrop({ left: value })}
                />
                <PixelInputField
                  label="T"
                  value={source.crop.top}
                  onChange={(value) => updateCrop({ top: value })}
                />
                <PixelInputField
                  label="R"
                  value={source.crop.right}
                  onChange={(value) => updateCrop({ right: value })}
                />
                <PixelInputField
                  label="B"
                  value={source.crop.bottom}
                  onChange={(value) => updateCrop({ bottom: value })}
                />
              </div>
            </div>

            <div className="grid grid-cols-2 gap-2">
              <Button
                className="gap-2"
                onClick={() => onChange(raiseSceneSource(scene, source.id))}
                size="sm"
                variant="outline"
              >
                <ArrowUp className="size-3" />
                Raise
              </Button>
              <Button
                className="gap-2"
                onClick={() => onChange(lowerSceneSource(scene, source.id))}
                size="sm"
                variant="outline"
              >
                <ArrowDown className="size-3" />
                Lower
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </ScrollArea>
  );
}

function formatDimension(value: number): string {
  return Math.round(value).toString();
}

function formatScale(value: number): string {
  return `${Math.round(value * 100)}%`;
}

function formatScaleLabel(
  scale: ReturnType<typeof sceneSourceScale>,
  preserveAspectRatio: boolean,
): string {
  if (!scale.scaleX || !scale.scaleY) {
    return 'unknown';
  }

  if (preserveAspectRatio) {
    return formatScale(scale.scale ?? scale.scaleX);
  }

  return `${formatScale(scale.scaleX)} x ${formatScale(scale.scaleY)}`;
}
