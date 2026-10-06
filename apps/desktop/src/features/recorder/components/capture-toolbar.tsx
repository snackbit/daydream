import type { ReactNode } from 'react';
import type {
  RecorderPreviewStatus,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import { LoaderCircle, Pause, Play, Save, Square } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { RecorderLifecycleAction } from '../recorder-workspace-store';
import { getCaptureStartBlocker } from './capture-toolbar/start-blocker';

export interface CaptureToolbarProps {
  configurationLocked: boolean;
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
  configurationLocked,
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
  const pauseDisabled = lifecycleLocked || status?.state !== 'recording';
  const resumeDisabled = lifecycleLocked || status?.state !== 'paused';
  const stopDisabled =
    lifecycleLocked ||
    (status?.state !== 'recording' && status?.state !== 'paused');
  const isRecording =
    status?.state === 'recording' || status?.state === 'pausing';
  const canStop = status?.state === 'recording' || status?.state === 'paused';
  const canShowStart =
    !isRecording && status?.state !== 'paused' && status?.state !== 'stopping';
  const startBlocker = canShowStart
    ? getCaptureStartBlocker({
        isDirty,
        lifecycleLocked,
        loadStatus,
        scene,
        status,
      })
    : null;
  const startDisabled = startBlocker !== null;

  return (
    <div className="bg-background flex shrink-0 flex-col gap-1.5 border-b px-3 py-2">
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
        <h1 className="text-sm font-semibold">Capture</h1>
        <div className="flex flex-wrap items-center gap-1.5">
          <Button
            className="gap-2"
            disabled={
              configurationLocked ||
              lifecycleLocked ||
              !scene ||
              !isDirty ||
              saveStatus === 'loading'
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
          {canShowStart ? (
            <Button
              className="gap-2"
              disabled={startDisabled}
              onClick={onStart}
            >
              <ToolbarButtonIcon
                action="start"
                activeAction={activeLifecycleAction}
                icon={<Play className="size-3.5" />}
              />
              {activeLifecycleAction === 'start' ? 'Starting' : 'Start'}
            </Button>
          ) : null}
          {isRecording ? (
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
          ) : null}
          {status?.state === 'paused' ? (
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
          ) : null}
          {canStop ? (
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
          ) : null}
        </div>
      </div>
      <div className="text-muted-foreground flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 text-xs">
        <span className="text-foreground/85 font-medium">
          Status: {status?.state ?? loadStatus}
        </span>
        <span>{isDirty ? 'Unsaved source setup' : 'Source setup saved'}</span>
        <span>Preview: {previewLabel(previewStatus)}</span>
        {startBlocker ? (
          <span aria-live="polite" className="text-foreground/90" role="status">
            {startBlocker}
          </span>
        ) : null}
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
