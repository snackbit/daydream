import type { AudioSource, RecorderScene } from '@daydream/shared';
import { Video } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { AudioSourceSelect } from './audio-source-select';
import { SceneSourceList } from './scene-source-list';

export interface CaptureSourcesPanelProps {
  audioSources: AudioSource[];
  desktopAudioSourceId: string | null;
  loadStatus: string;
  microphoneAudioSourceId: string | null;
  onSelectAudioSource: (
    role: 'desktop-audio' | 'microphone',
    sourceId: string | null,
  ) => void;
  onDeleteSceneSource: (sceneSourceId: string) => void;
  onSelectCaptureSources: () => void;
  onSelectSceneSource: (sourceId: string | null) => void;
  scene: RecorderScene | null;
  selectedSceneSourceId: string | null;
}

export function CaptureSourcesPanel({
  audioSources,
  desktopAudioSourceId,
  loadStatus,
  microphoneAudioSourceId,
  onSelectAudioSource,
  onDeleteSceneSource,
  onSelectCaptureSources,
  onSelectSceneSource,
  scene,
  selectedSceneSourceId,
}: CaptureSourcesPanelProps) {
  return (
    <ScrollArea className="h-full">
      <div className="p-4">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-sm font-medium">Sources</h2>
          <Button
            className="gap-2"
            disabled={loadStatus === 'loading'}
            onClick={onSelectCaptureSources}
            size="sm"
            variant="outline"
          >
            <Video className="size-3" />
            Add
          </Button>
        </div>

        <div className="mt-3">
          <SceneSourceList
            selectedSceneSourceId={selectedSceneSourceId}
            sources={scene?.sources ?? []}
            onDelete={onDeleteSceneSource}
            onSelect={onSelectSceneSource}
          />
        </div>

        <Separator className="my-5" />

        <h2 className="text-sm font-medium">Audio</h2>
        <div className="mt-3 grid gap-3">
          <AudioSourceSelect
            label="Desktop"
            role="desktop-audio"
            selectedSourceId={desktopAudioSourceId}
            sources={audioSources}
            onChange={(sourceId) =>
              onSelectAudioSource('desktop-audio', sourceId)
            }
          />
          <AudioSourceSelect
            label="Microphone"
            role="microphone"
            selectedSourceId={microphoneAudioSourceId}
            sources={audioSources}
            onChange={(sourceId) => onSelectAudioSource('microphone', sourceId)}
          />
        </div>
      </div>
    </ScrollArea>
  );
}
