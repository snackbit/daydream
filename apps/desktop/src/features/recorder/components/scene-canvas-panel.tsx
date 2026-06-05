import type { RecorderScene } from '@daydream/shared';
import { Card, CardContent } from '@/components/ui/card';
import { SceneCanvas } from './scene-canvas';

export interface SceneCanvasPanelProps {
  onChange: (scene: RecorderScene) => void;
  onSelect: (sourceId: string | null) => void;
  scene: RecorderScene | null;
  selectedSourceId: string | null;
}

export function SceneCanvasPanel({
  onChange,
  onSelect,
  scene,
  selectedSourceId,
}: SceneCanvasPanelProps) {
  if (!scene) {
    return (
      <Card>
        <CardContent className="text-muted-foreground text-sm">
          Loading recorder workspace.
        </CardContent>
      </Card>
    );
  }

  return (
    <SceneCanvas
      scene={scene}
      selectedSourceId={selectedSourceId}
      onChange={onChange}
      onSelect={onSelect}
    />
  );
}
