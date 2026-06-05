import { useMemo, type SyntheticEvent } from 'react';
import type {
  AudioSource,
  AudioSourceRole,
  CaptureSource,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import type {
  DuplicateCaptureSourceCandidate,
  DuplicateCaptureSourceResolution,
} from '@/features/recorder/scene-layout';
import type { RecorderLifecycleAction } from '../recorder-workspace-store';
import { CaptureResizableLayout } from './capture-resizable-layout';
import { CaptureToolbar } from './capture-toolbar';
import { RecorderErrorAlert } from './recorder-error-alert';

export interface CaptureWorkspaceProps {
  audioSources: AudioSource[];
  captureSources: CaptureSource[];
  desktopAudioSourceId: string | null;
  error: string | null;
  isDirty: boolean;
  loadStatus: string;
  microphoneAudioSourceId: string | null;
  onChangeScene: (scene: RecorderScene) => void;
  onDeleteSceneSource: (sceneSourceId: string) => void;
  onPause: () => void;
  onResume: () => void;
  onResolveDuplicateCaptureSource: (
    resolution: DuplicateCaptureSourceResolution,
  ) => void;
  onSave: () => void;
  onSelectAudioSource: (role: AudioSourceRole, sourceId: string | null) => void;
  onSelectCaptureSources: () => void;
  onSelectSceneSource: (sourceId: string | null) => void;
  onStart: () => void;
  onStop: () => void;
  pendingLifecycleAction: RecorderLifecycleAction | null;
  saveStatus: string;
  scene: RecorderScene | null;
  pendingDuplicateCaptureSources: DuplicateCaptureSourceCandidate[];
  selectedSceneSourceId: string | null;
  status: RecorderStatus | null;
}

export function CaptureWorkspace({
  audioSources,
  captureSources,
  desktopAudioSourceId,
  error,
  isDirty,
  loadStatus,
  microphoneAudioSourceId,
  onChangeScene,
  onDeleteSceneSource,
  onPause,
  onResume,
  onResolveDuplicateCaptureSource,
  onSave,
  onSelectAudioSource,
  onSelectCaptureSources,
  onSelectSceneSource,
  onStart,
  onStop,
  pendingLifecycleAction,
  saveStatus,
  scene,
  pendingDuplicateCaptureSources,
  selectedSceneSourceId,
  status,
}: CaptureWorkspaceProps) {
  const selectedSceneSource = useMemo(
    () =>
      scene?.sources.find((source) => source.id === selectedSceneSourceId) ??
      null,
    [scene, selectedSceneSourceId],
  );
  const lifecycleLocked =
    pendingLifecycleAction !== null ||
    status?.state === 'pausing' ||
    status?.state === 'selecting' ||
    status?.state === 'stopping';

  return (
    <section className="flex min-h-full flex-col">
      <CaptureToolbar
        isDirty={isDirty}
        lifecycleLocked={lifecycleLocked}
        loadStatus={loadStatus}
        pendingLifecycleAction={pendingLifecycleAction}
        saveStatus={saveStatus}
        scene={scene}
        status={status}
        onPause={onPause}
        onResume={onResume}
        onSave={onSave}
        onStart={onStart}
        onStop={onStop}
      />

      {error ? <RecorderErrorAlert message={error} /> : null}

      <div
        aria-disabled={lifecycleLocked}
        className={
          lifecycleLocked ? 'min-h-0 flex-1 opacity-60' : 'min-h-0 flex-1'
        }
        onClickCapture={lifecycleLocked ? preventLockedInteraction : undefined}
        onKeyDownCapture={
          lifecycleLocked ? preventLockedInteraction : undefined
        }
        onPointerDownCapture={
          lifecycleLocked ? preventLockedInteraction : undefined
        }
      >
        <CaptureResizableLayout
          audioSources={audioSources}
          captureSources={captureSources}
          desktopAudioSourceId={desktopAudioSourceId}
          lifecycleLocked={lifecycleLocked}
          loadStatus={loadStatus}
          microphoneAudioSourceId={microphoneAudioSourceId}
          pendingDuplicateCaptureSources={pendingDuplicateCaptureSources}
          scene={scene}
          selectedSceneSource={selectedSceneSource}
          selectedSceneSourceId={selectedSceneSourceId}
          onChangeScene={onChangeScene}
          onDeleteSceneSource={onDeleteSceneSource}
          onSelectAudioSource={onSelectAudioSource}
          onSelectCaptureSources={onSelectCaptureSources}
          onSelectSceneSource={onSelectSceneSource}
          onResolveDuplicateCaptureSource={onResolveDuplicateCaptureSource}
        />
      </div>
    </section>
  );
}

function preventLockedInteraction(event: SyntheticEvent) {
  event.preventDefault();
  event.stopPropagation();
}
