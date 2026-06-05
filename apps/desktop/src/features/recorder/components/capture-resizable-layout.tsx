import type {
  AudioSource,
  AudioSourceRole,
  CaptureSource,
  RecorderScene,
  RecorderSceneSource,
} from '@daydream/shared';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import type {
  DuplicateCaptureSourceCandidate,
  DuplicateCaptureSourceResolution,
} from '@/features/recorder/scene-layout';
import { CaptureSourcesPanel } from './capture-sources-panel';
import { GeometryPanel } from './geometry-panel';
import { SceneCanvasPanel } from './scene-canvas-panel';

export interface CaptureResizableLayoutProps {
  audioSources: AudioSource[];
  captureSources: CaptureSource[];
  desktopAudioSourceId: string | null;
  lifecycleLocked: boolean;
  loadStatus: string;
  microphoneAudioSourceId: string | null;
  onChangeScene: (scene: RecorderScene) => void;
  onDeleteSceneSource: (sceneSourceId: string) => void;
  onResolveDuplicateCaptureSource: (
    resolution: DuplicateCaptureSourceResolution,
  ) => void;
  onSelectAudioSource: (role: AudioSourceRole, sourceId: string | null) => void;
  onSelectCaptureSources: () => void;
  onSelectSceneSource: (sourceId: string | null) => void;
  scene: RecorderScene | null;
  pendingDuplicateCaptureSources: DuplicateCaptureSourceCandidate[];
  selectedSceneSource: RecorderSceneSource | null;
  selectedSceneSourceId: string | null;
}

export function CaptureResizableLayout({
  audioSources,
  captureSources,
  desktopAudioSourceId,
  lifecycleLocked,
  loadStatus,
  microphoneAudioSourceId,
  onChangeScene,
  onDeleteSceneSource,
  onResolveDuplicateCaptureSource,
  onSelectAudioSource,
  onSelectCaptureSources,
  onSelectSceneSource,
  scene,
  pendingDuplicateCaptureSources,
  selectedSceneSource,
  selectedSceneSourceId,
}: CaptureResizableLayoutProps) {
  return (
    <ResizablePanelGroup className="min-h-0 flex-1" orientation="horizontal">
      <ResizablePanel defaultSize="22%" minSize="16%">
        <CaptureSourcesPanel
          audioSources={audioSources}
          desktopAudioSourceId={desktopAudioSourceId}
          lifecycleLocked={lifecycleLocked}
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
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel defaultSize="52%" minSize="32%">
        <main className="bg-foreground/10 flex h-full flex-col items-center justify-center overflow-auto p-4">
          <div className="bg-foreground/10 w-full">
            <SceneCanvasPanel
              scene={scene}
              selectedSourceId={selectedSceneSourceId}
              onChange={onChangeScene}
              onSelect={onSelectSceneSource}
            />
          </div>
        </main>
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel defaultSize="26%" minSize="20%">
        <GeometryPanel
          captureSources={captureSources}
          scene={scene}
          source={selectedSceneSource}
          onChange={onChangeScene}
        />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
