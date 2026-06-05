import type { ReactNode } from 'react';
import type { RecorderScene } from '@daydream/shared';
import { Columns2, Grid2X2, Rows2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  NativeSelect,
  NativeSelectOption,
} from '@/components/ui/native-select';
import {
  applySceneLayout,
  isSceneUsingAutoCanvas,
  SCENE_CANVAS_PRESETS,
  setSceneCanvasSize,
} from '@/features/recorder/scene-layout';
import type { SceneLayoutMode } from '@/features/recorder/scene-layout';

const canvasPresets = [
  {
    label: 'Auto',
    value: 'auto',
    width: null,
    height: null,
  },
  {
    label: '1920 x 1080',
    value: '1920x1080',
    ...SCENE_CANVAS_PRESETS[0],
  },
  {
    label: '2560 x 1440',
    value: '2560x1440',
    ...SCENE_CANVAS_PRESETS[1],
  },
  {
    label: '3840 x 2160',
    value: '3840x2160',
    ...SCENE_CANVAS_PRESETS[2],
  },
] as const;

export interface SceneControlsProps {
  onChange: (scene: RecorderScene) => void;
  scene: RecorderScene;
}

export function SceneControls({ onChange, scene }: SceneControlsProps) {
  const selectedPreset = isSceneUsingAutoCanvas(scene)
    ? 'auto'
    : (canvasPresets.find(
        (preset) =>
          preset.width === scene.canvasWidth &&
          preset.height === scene.canvasHeight,
      )?.value ?? 'custom');

  return (
    <div className="bg-background flex w-full flex-wrap items-center justify-between gap-2 border-b px-3 py-2">
      <div className="flex flex-wrap items-center gap-1">
        <LayoutButton
          active={scene.layoutMode === 'auto-16-9'}
          label="Auto layout"
          mode="auto-16-9"
          scene={scene}
          onChange={onChange}
        >
          <Grid2X2 className="size-3" />
        </LayoutButton>
        <LayoutButton
          active={scene.layoutMode === 'horizontal'}
          label="Horizontal layout"
          mode="horizontal"
          scene={scene}
          onChange={onChange}
        >
          <Columns2 className="size-3" />
        </LayoutButton>
        <LayoutButton
          active={scene.layoutMode === 'vertical'}
          label="Vertical layout"
          mode="vertical"
          scene={scene}
          onChange={onChange}
        >
          <Rows2 className="size-3" />
        </LayoutButton>
      </div>

      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <NativeSelect
          aria-label="Canvas preset"
          size="sm"
          value={selectedPreset}
          onChange={(event) => {
            const value = event.target.value;
            if (value === 'auto') {
              onChange(
                applySceneLayout(
                  scene,
                  scene.layoutMode === 'custom'
                    ? 'auto-16-9'
                    : scene.layoutMode,
                ),
              );
              return;
            }

            const preset = canvasPresets.find(
              (candidate) => candidate.value === value,
            );
            if (!preset || preset.width === null || preset.height === null) {
              return;
            }

            onChange(setSceneCanvasSize(scene, preset.width, preset.height));
          }}
        >
          {canvasPresets.map((preset) => (
            <NativeSelectOption key={preset.value} value={preset.value}>
              {preset.label}
            </NativeSelectOption>
          ))}
          <NativeSelectOption value="custom">Custom</NativeSelectOption>
        </NativeSelect>

        <CanvasDimensionInput
          label="W"
          value={scene.canvasWidth}
          onChange={(width) =>
            onChange(setSceneCanvasSize(scene, width, scene.canvasHeight))
          }
        />
        <CanvasDimensionInput
          label="H"
          value={scene.canvasHeight}
          onChange={(height) =>
            onChange(setSceneCanvasSize(scene, scene.canvasWidth, height))
          }
        />
      </div>
    </div>
  );
}

interface LayoutButtonProps {
  active: boolean;
  children: ReactNode;
  label: string;
  mode: SceneLayoutMode;
  onChange: (scene: RecorderScene) => void;
  scene: RecorderScene;
}

function LayoutButton({
  active,
  children,
  label,
  mode,
  onChange,
  scene,
}: LayoutButtonProps) {
  return (
    <Button
      aria-label={label}
      size="icon-sm"
      type="button"
      variant={active ? 'default' : 'outline'}
      onClick={() =>
        onChange(
          isSceneUsingAutoCanvas(scene)
            ? applySceneLayout(scene, mode)
            : applySceneLayout(scene, mode, {
                height: scene.canvasHeight,
                width: scene.canvasWidth,
              }),
        )
      }
    >
      {children}
    </Button>
  );
}

interface CanvasDimensionInputProps {
  label: string;
  onChange: (value: number) => void;
  value: number;
}

function CanvasDimensionInput({
  label,
  onChange,
  value,
}: CanvasDimensionInputProps) {
  return (
    <label className="text-muted-foreground flex items-center gap-1 text-xs">
      {label}
      <Input
        aria-label={`Canvas ${label}`}
        className="h-6 w-20 text-xs"
        min={1}
        type="number"
        value={Math.round(value)}
        onChange={(event) => onChange(event.target.valueAsNumber)}
      />
    </label>
  );
}
