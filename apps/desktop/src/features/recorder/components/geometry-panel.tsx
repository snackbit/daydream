import type {
  CanvasRect,
  CropRect,
  RecorderScene,
  RecorderSceneSource,
} from '@daydream/shared';
import { ArrowDown, ArrowUp, Crop } from 'lucide-react';
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
  lowerSceneSource,
  raiseSceneSource,
  setSceneSourceAspectLock,
  setSceneSourceCrop,
  setSceneSourceRect,
} from '@/features/recorder/scene-layout';
import { PixelInputField } from './pixel-input-field';

export interface GeometryPanelProps {
  onChange: (scene: RecorderScene) => void;
  scene: RecorderScene | null;
  source: RecorderSceneSource | null;
}

export function GeometryPanel({ onChange, scene, source }: GeometryPanelProps) {
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

  const updateRect = (partial: Partial<CanvasRect>) => {
    onChange(
      setSceneSourceRect(scene, source.id, {
        ...source.rect,
        ...partial,
      }),
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
                onChange={(value) => updateRect({ width: value })}
              />
              <PixelInputField
                label="H"
                value={source.rect.height}
                onChange={(value) => updateRect({ height: value })}
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
