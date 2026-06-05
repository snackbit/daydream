import type { RecorderPreviewFrame, RecorderScene } from '@daydream/shared';
import { Card, CardContent } from '@/components/ui/card';
import { SceneCanvas } from './scene-canvas';
import { SceneControls } from './scene-controls';

export interface SceneCanvasPanelProps {
  onChange: (scene: RecorderScene) => void;
  onSelect: (sourceId: string | null) => void;
  previewFramesBySceneSourceId: Record<string, RecorderPreviewFrame>;
  scene: RecorderScene | null;
  selectedSourceId: string | null;
}

export function SceneCanvasPanel({
  onChange,
  onSelect,
  previewFramesBySceneSourceId,
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
    <div className="flex h-full w-full flex-col">
      <SceneControls scene={scene} onChange={onChange} />
      <div className="min-h-0 flex-1 p-4">
        <SceneCanvas
          previewFramesBySceneSourceId={previewFramesBySceneSourceId}
          scene={scene}
          selectedSourceId={selectedSourceId}
          onChange={onChange}
          onSelect={onSelect}
        />
      </div>
    </div>
  );
}
