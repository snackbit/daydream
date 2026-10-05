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
  disabled?: boolean;
  onChange: (scene: RecorderScene) => void;
  scene: RecorderScene | null;
  source: RecorderSceneSource | null;
}

export function GeometryPanel({
  captureSources,
  disabled = false,
  onChange,
  scene,
  source,
}: GeometryPanelProps) {
  if (!scene || !source) {
    return (
      <ScrollArea className="h-full">
        <div className="p-3">
          <h2 className="text-xs font-semibold">Geometry</h2>
          <p className="text-muted-foreground mt-2 border-y py-2 text-xs">
            Select a monitor to edit its layout.
          </p>
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
      <div className="p-3">
        <div className="flex items-center justify-between gap-2 border-b pb-2">
          <h2 className="text-xs font-semibold">Geometry</h2>
          <span className="text-muted-foreground min-w-0 truncate font-mono text-[10px]">
            {source.displayName} · {source.sourceId}
          </span>
        </div>
        <div className="grid gap-3 pt-2">
          <div className="text-muted-foreground grid gap-1 text-[10px]">
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
              disabled={disabled}
              label="X"
              value={source.rect.x}
              onChange={(value) => updateRect({ x: value })}
            />
            <PixelInputField
              disabled={disabled}
              label="Y"
              value={source.rect.y}
              onChange={(value) => updateRect({ y: value })}
            />
            <PixelInputField
              disabled={disabled}
              label="W"
              value={source.rect.width}
              onChange={(value) => updateRect({ width: value }, 'width')}
            />
            <PixelInputField
              disabled={disabled}
              label="H"
              value={source.rect.height}
              onChange={(value) => updateRect({ height: value }, 'height')}
            />
          </div>

          <Field orientation="horizontal">
            <Checkbox
              checked={source.preserveAspectRatio}
              disabled={disabled}
              id={aspectInputId}
              onCheckedChange={(checked) =>
                onChange(
                  setSceneSourceAspectLock(scene, source.id, checked === true),
                )
              }
            />
            <FieldContent>
              <FieldLabel htmlFor={aspectInputId}>Aspect</FieldLabel>
              <FieldDescription>Preserve ratio while resizing</FieldDescription>
            </FieldContent>
          </Field>

          <div className="grid grid-cols-2 gap-2">
            <Button
              className="gap-2"
              disabled={disabled}
              onClick={() => onChange(fitSceneSourceToCanvas(scene, source.id))}
              size="sm"
              variant="outline"
            >
              <Minimize2 className="size-3" />
              Fit
            </Button>
            <Button
              className="gap-2"
              disabled={disabled}
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
              disabled={disabled}
              onClick={() => onChange(centerSceneSource(scene, source.id))}
              size="sm"
              variant="outline"
            >
              <Crosshair className="size-3" />
              Center
            </Button>
            <Button
              className="gap-2"
              disabled={disabled}
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
                disabled={disabled}
                label="Left"
                value={source.crop.left}
                onChange={(value) => updateCrop({ left: value })}
              />
              <PixelInputField
                disabled={disabled}
                label="Top"
                value={source.crop.top}
                onChange={(value) => updateCrop({ top: value })}
              />
              <PixelInputField
                disabled={disabled}
                label="Right"
                value={source.crop.right}
                onChange={(value) => updateCrop({ right: value })}
              />
              <PixelInputField
                disabled={disabled}
                label="Bottom"
                value={source.crop.bottom}
                onChange={(value) => updateCrop({ bottom: value })}
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-2">
            <Button
              className="gap-2"
              disabled={disabled}
              onClick={() => onChange(raiseSceneSource(scene, source.id))}
              size="sm"
              variant="outline"
            >
              <ArrowUp className="size-3" />
              Raise
            </Button>
            <Button
              className="gap-2"
              disabled={disabled}
              onClick={() => onChange(lowerSceneSource(scene, source.id))}
              size="sm"
              variant="outline"
            >
              <ArrowDown className="size-3" />
              Lower
            </Button>
          </div>
        </div>
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
