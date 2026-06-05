import type {
  AudioSource,
  RecorderScene,
  RecorderSceneSource,
} from '@daydream/shared';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { CaptureSourcesPanel } from './capture-sources-panel';
import { GeometryPanel } from './geometry-panel';
import { SceneCanvasPanel } from './scene-canvas-panel';

export interface CaptureResizableLayoutProps {
  audioSources: AudioSource[];
  desktopAudioSourceId: string | null;
  loadStatus: string;
  microphoneAudioSourceId: string | null;
  onChangeScene: (scene: RecorderScene) => void;
  onDeleteSceneSource: (sceneSourceId: string) => void;
  onSelectAudioSource: (
    role: 'desktop-audio' | 'microphone',
    sourceId: string | null,
  ) => void;
  onSelectCaptureSources: () => void;
  onSelectSceneSource: (sourceId: string | null) => void;
  scene: RecorderScene | null;
  selectedSceneSource: RecorderSceneSource | null;
  selectedSceneSourceId: string | null;
}

export function CaptureResizableLayout({
  audioSources,
  desktopAudioSourceId,
  loadStatus,
  microphoneAudioSourceId,
  onChangeScene,
  onDeleteSceneSource,
  onSelectAudioSource,
  onSelectCaptureSources,
  onSelectSceneSource,
  scene,
  selectedSceneSource,
  selectedSceneSourceId,
}: CaptureResizableLayoutProps) {
  return (
    <ResizablePanelGroup className="min-h-0 flex-1" orientation="horizontal">
      <ResizablePanel defaultSize="22%" minSize="16%">
        <CaptureSourcesPanel
          audioSources={audioSources}
          desktopAudioSourceId={desktopAudioSourceId}
          loadStatus={loadStatus}
          microphoneAudioSourceId={microphoneAudioSourceId}
          scene={scene}
          selectedSceneSourceId={selectedSceneSourceId}
          onDeleteSceneSource={onDeleteSceneSource}
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
          scene={scene}
          source={selectedSceneSource}
          onChange={onChangeScene}
        />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
