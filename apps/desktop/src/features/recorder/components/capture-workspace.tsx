import { useMemo, type SyntheticEvent } from 'react';
import type {
  SaveCaptureSetupInput,
  AudioSource,
  AudioSourceRole,
  CaptureSource,
  RecorderPreviewFrame,
  RecorderPreviewStatus,
  RecorderRecordingStats,
  RecorderScene,
  RecorderStatus,
} from '@daydream/shared';
import { LoaderCircle } from 'lucide-react';
import { FullHeightLayout } from '@/app/PageLayout';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import type {
  DuplicateCaptureSourceCandidate,
  DuplicateCaptureSourceResolution,
} from '@/features/recorder/scene-layout';
import { cn } from '@/lib/utils';
import { recorderConfigurationLocked } from '../recorder-lifecycle-state';
import type { RecorderLifecycleAction } from '../recorder-workspace-store';
import { CaptureResizableLayout } from './capture-resizable-layout';
import { CaptureSourcesPanel } from './capture-sources-panel';
import { CaptureToolbar } from './capture-toolbar';
import { GeometryPanel } from './geometry-panel';
import { RecorderErrorAlert } from './recorder-error-alert';
import { SceneCanvasPanel } from './scene-canvas-panel';

export interface CaptureWorkspaceProps {
  captureSelectionPending?: boolean;
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
  onStart: (setup?: SaveCaptureSetupInput) => void;
  onStop: () => void;
  pendingLifecycleAction: RecorderLifecycleAction | null;
  previewFramesBySceneSourceId: Record<string, RecorderPreviewFrame>;
  previewStatus: RecorderPreviewStatus | null;
  recordingStats: RecorderRecordingStats | null;
  recordingStatsError: string | null;
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
  previewFramesBySceneSourceId,
  previewStatus,
  recordingStats,
  recordingStatsError,
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
    status?.finalization != null ||
    status?.state === 'pausing' ||
    status?.state === 'selecting' ||
    status?.state === 'stopping';
  const workspaceLocked =
    pendingLifecycleAction !== null ||
    status?.finalization != null ||
    status?.state === 'pausing' ||
    status?.state === 'selecting' ||
    status?.state === 'stopping';
  const configurationLocked = recorderConfigurationLocked(
    pendingLifecycleAction,
    status,
  );

  return (
    <FullHeightLayout>
      <CaptureToolbar
        configurationLocked={configurationLocked}
        isDirty={isDirty}
        lifecycleLocked={lifecycleLocked}
        loadStatus={loadStatus}
        pendingLifecycleAction={pendingLifecycleAction}
        previewStatus={previewStatus}
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
      {status?.finalization ? (
        <div className="px-4 pt-4">
          <Alert>
            <LoaderCircle className="animate-spin" />
            <AlertTitle>
              {status.finalization.action === 'stop'
                ? 'Saving recording'
                : 'Saving segment'}
            </AlertTitle>
            <AlertDescription>
              Capture is stopping and local metadata is being finalized.
            </AlertDescription>
          </Alert>
        </div>
      ) : null}

      <div
        aria-disabled={workspaceLocked}
        className={cn(
          'flex min-h-0 flex-1 flex-col',
          workspaceLocked && 'opacity-60',
        )}
        onClickCapture={workspaceLocked ? preventLockedInteraction : undefined}
        onKeyDownCapture={
          workspaceLocked ? preventLockedInteraction : undefined
        }
        onPointerDownCapture={
          workspaceLocked ? preventLockedInteraction : undefined
        }
      >
        <CaptureResizableLayout
          canvasPanel={
            <SceneCanvasPanel
              configurationLocked={configurationLocked}
              previewFramesBySceneSourceId={previewFramesBySceneSourceId}
              recordingStats={recordingStats}
              recordingStatsError={recordingStatsError}
              scene={scene}
              selectedSourceId={selectedSceneSourceId}
              status={status}
              onChange={onChangeScene}
              onSelect={onSelectSceneSource}
            />
          }
          geometryPanel={
            <GeometryPanel
              captureSources={captureSources}
              disabled={configurationLocked}
              scene={scene}
              source={selectedSceneSource}
              onChange={onChangeScene}
            />
          }
          sourcesPanel={
            <CaptureSourcesPanel
              audioSources={audioSources}
              configurationLocked={configurationLocked}
              desktopAudioSourceId={desktopAudioSourceId}
              lifecycleLocked={workspaceLocked}
              loadStatus={loadStatus}
              microphoneAudioSourceId={microphoneAudioSourceId}
              pendingDuplicateCaptureSources={pendingDuplicateCaptureSources}
              scene={scene}
              selectedSceneSourceId={selectedSceneSourceId}
              onDeleteSceneSource={onDeleteSceneSource}
              onResolveDuplicateCaptureSource={onResolveDuplicateCaptureSource}
              onSelectAudioSource={onSelectAudioSource}
              onSelectCaptureSources={onSelectCaptureSources}
              onSelectSceneSource={onSelectSceneSource}
            />
          }
        />
      </div>
    </FullHeightLayout>
  );
}

function preventLockedInteraction(event: SyntheticEvent) {
  event.preventDefault();
  event.stopPropagation();
}
