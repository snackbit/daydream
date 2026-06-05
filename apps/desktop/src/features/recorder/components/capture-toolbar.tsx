import type { RecorderScene, RecorderStatus } from '@daydream/shared';
import { Pause, Play, Save, Square } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

export interface CaptureToolbarProps {
  isDirty: boolean;
  loadStatus: string;
  onPause: () => void;
  onResume: () => void;
  onSave: () => void;
  onStart: () => void;
  onStop: () => void;
  saveStatus: string;
  scene: RecorderScene | null;
  status: RecorderStatus | null;
}

export function CaptureToolbar({
  isDirty,
  loadStatus,
  onPause,
  onResume,
  onSave,
  onStart,
  onStop,
  saveStatus,
  scene,
  status,
}: CaptureToolbarProps) {
  const startDisabled =
    !scene ||
    scene.sources.length === 0 ||
    isDirty ||
    status?.state !== 'idle' ||
    !status.backendHealth.portal.available ||
    !status.backendHealth.pipewire.available ||
    !status.backendHealth.ffmpeg.available;
  const pauseDisabled = status?.state !== 'recording';
  const resumeDisabled = status?.state !== 'paused';
  const stopDisabled =
    status?.state !== 'recording' && status?.state !== 'paused';

  return (
    <div className="flex shrink-0 items-start justify-between gap-4 border-b px-6 py-4">
      <div>
        <h1 className="text-xl font-semibold tracking-normal">Capture</h1>
        <div className="mt-2 flex flex-wrap gap-2">
          <Badge variant="secondary">
            Status: {status?.state ?? loadStatus}
          </Badge>
          <Badge variant="outline">FPS: 1</Badge>
          <Badge variant="outline">Quality: balanced-ocr</Badge>
          <Badge variant={isDirty ? 'destructive' : 'secondary'}>
            {isDirty ? 'Unsaved scene' : 'Scene saved'}
          </Badge>
        </div>
      </div>

      <div className="flex flex-wrap justify-end gap-2">
        <Button
          className="gap-2"
          disabled={!scene || !isDirty || saveStatus === 'loading'}
          onClick={onSave}
          variant="outline"
        >
          <Save className="size-3.5" />
          Save
        </Button>
        <Button className="gap-2" disabled={startDisabled} onClick={onStart}>
          <Play className="size-3.5" />
          Start
        </Button>
        <Button
          className="gap-2"
          disabled={pauseDisabled}
          onClick={onPause}
          variant="outline"
        >
          <Pause className="size-3.5" />
          Pause
        </Button>
        <Button
          className="gap-2"
          disabled={resumeDisabled}
          onClick={onResume}
          variant="outline"
        >
          <Play className="size-3.5" />
          Resume
        </Button>
        <Button
          className="gap-2"
          disabled={stopDisabled}
          onClick={onStop}
          variant="destructive"
        >
          <Square className="size-3.5" />
          Stop
        </Button>
      </div>
    </div>
  );
}
