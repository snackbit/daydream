import type { ReactNode } from 'react';
import type {
  RecorderPreviewStatus,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import { LoaderCircle, Pause, Play, Save, Square } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { RecorderLifecycleAction } from '../recorder-workspace-store';

export interface CaptureToolbarProps {
  isDirty: boolean;
  lifecycleLocked: boolean;
  loadStatus: string;
  onPause: () => void;
  onResume: () => void;
  onSave: () => void;
  onStart: () => void;
  onStop: () => void;
  pendingLifecycleAction: RecorderLifecycleAction | null;
  previewStatus: RecorderPreviewStatus | null;
  saveStatus: string;
  scene: RecorderScene | null;
  status: RecorderStatus | null;
}

export function CaptureToolbar({
  isDirty,
  lifecycleLocked,
  loadStatus,
  onPause,
  onResume,
  onSave,
  onStart,
  onStop,
  pendingLifecycleAction,
  previewStatus,
  saveStatus,
  scene,
  status,
}: CaptureToolbarProps) {
  const activeLifecycleAction =
    pendingLifecycleAction ?? lifecycleActionFromStatus(status);
  const startDisabled =
    lifecycleLocked ||
    !scene ||
    scene.sources.length === 0 ||
    isDirty ||
    status?.state !== 'idle' ||
    !status.backendHealth.portal.available ||
    !status.backendHealth.pipewire.available ||
    !status.backendHealth.ffmpeg.available;
  const pauseDisabled = lifecycleLocked || status?.state !== 'recording';
  const resumeDisabled = lifecycleLocked || status?.state !== 'paused';
  const stopDisabled =
    lifecycleLocked ||
    (status?.state !== 'recording' && status?.state !== 'paused');

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
          <Badge
            variant={
              previewStatus?.state === 'error' ? 'destructive' : 'outline'
            }
          >
            Preview: {previewLabel(previewStatus)}
          </Badge>
        </div>
      </div>

      <div className="flex flex-wrap justify-end gap-2">
        <Button
          className="gap-2"
          disabled={
            lifecycleLocked || !scene || !isDirty || saveStatus === 'loading'
          }
          onClick={onSave}
          variant="outline"
        >
          {saveStatus === 'loading' ? (
            <LoaderCircle className="size-3.5 animate-spin" />
          ) : (
            <Save className="size-3.5" />
          )}
          Save
        </Button>
        <Button className="gap-2" disabled={startDisabled} onClick={onStart}>
          <ToolbarButtonIcon
            action="start"
            activeAction={activeLifecycleAction}
            icon={<Play className="size-3.5" />}
          />
          {activeLifecycleAction === 'start' ? 'Starting' : 'Start'}
        </Button>
        <Button
          className="gap-2"
          disabled={pauseDisabled}
          onClick={onPause}
          variant="outline"
        >
          <ToolbarButtonIcon
            action="pause"
            activeAction={activeLifecycleAction}
            icon={<Pause className="size-3.5" />}
          />
          {activeLifecycleAction === 'pause' ? 'Pausing' : 'Pause'}
        </Button>
        <Button
          className="gap-2"
          disabled={resumeDisabled}
          onClick={onResume}
          variant="outline"
        >
          <ToolbarButtonIcon
            action="resume"
            activeAction={activeLifecycleAction}
            icon={<Play className="size-3.5" />}
          />
          {activeLifecycleAction === 'resume' ? 'Resuming' : 'Resume'}
        </Button>
        <Button
          className="gap-2"
          disabled={stopDisabled}
          onClick={onStop}
          variant="destructive"
        >
          <ToolbarButtonIcon
            action="stop"
            activeAction={activeLifecycleAction}
            icon={<Square className="size-3.5" />}
          />
          {activeLifecycleAction === 'stop' ? 'Stopping' : 'Stop'}
        </Button>
      </div>
    </div>
  );
}

function previewLabel(status: RecorderPreviewStatus | null): string {
  if (!status) {
    return 'stopped';
  }

  switch (status.state) {
    case 'error':
      return 'unavailable';
    case 'idle':
      return 'stopped';
    case 'live':
      return 'live';
    case 'starting':
      return 'starting';
  }
}

function lifecycleActionFromStatus(
  status: RecorderStatus | null,
): RecorderLifecycleAction | null {
  if (status?.state === 'pausing') {
    return 'pause';
  }
  if (status?.state === 'selecting') {
    return 'start';
  }
  if (status?.state === 'stopping') {
    return 'stop';
  }

  return null;
}

function ToolbarButtonIcon({
  action,
  activeAction,
  icon,
}: {
  action: RecorderLifecycleAction;
  activeAction: RecorderLifecycleAction | null;
  icon: ReactNode;
}) {
  return activeAction === action ? (
    <LoaderCircle className="size-3.5 animate-spin" />
  ) : (
    icon
  );
}
