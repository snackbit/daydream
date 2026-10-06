import type {
  RecorderPreviewFrame,
  RecorderRecordingStats,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import { RecordingInProgressPanel } from './recording-in-progress-panel';
import { SceneCanvas } from './scene-canvas';
import { SceneControls } from './scene-controls';

export interface SceneCanvasPanelProps {
  configurationLocked?: boolean;
  onChange: (scene: RecorderScene) => void;
  onSelect: (sourceId: string | null) => void;
  previewFramesBySceneSourceId: Record<string, RecorderPreviewFrame>;
  recordingStats: RecorderRecordingStats | null;
  recordingStatsError: string | null;
  scene: RecorderScene | null;
  selectedSourceId: string | null;
  status: RecorderStatus | null;
}

export function SceneCanvasPanel({
  configurationLocked = false,
  onChange,
  onSelect,
  previewFramesBySceneSourceId,
  recordingStats,
  recordingStatsError,
  scene,
  selectedSourceId,
  status,
}: SceneCanvasPanelProps) {
  if (!scene) {
    return (
      <div className="text-muted-foreground p-3 text-xs" role="status">
        Loading recorder workspace.
      </div>
    );
  }

  if (status?.state === 'recording') {
    return (
      <RecordingInProgressPanel
        recordingStats={recordingStats}
        recordingStatsError={recordingStatsError}
      />
    );
  }

  return (
    <div className="flex h-full w-full flex-col">
      <SceneControls
        disabled={configurationLocked}
        scene={scene}
        onChange={onChange}
      />
      <div className="min-h-0 flex-1 p-4">
        <SceneCanvas
          previewFramesBySceneSourceId={previewFramesBySceneSourceId}
          readOnly={configurationLocked}
          scene={scene}
          selectedSourceId={selectedSourceId}
          onChange={onChange}
          onSelect={onSelect}
        />
      </div>
    </div>
  );
}
