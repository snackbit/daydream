import type {
  AudioSource,
  AudioSourceRole,
  RecorderScene,
} from '@daydream/shared';
import { Video, Volume2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import type {
  DuplicateCaptureSourceCandidate,
  DuplicateCaptureSourceResolution,
} from '@/features/recorder/scene-layout';
import { AudioSourceList, AudioSourcePickerDialog } from './audio-source-list';
import { SceneSourceList } from './scene-source-list';

export interface CaptureSourcesPanelProps {
  audioSources: AudioSource[];
  desktopAudioSourceId: string | null;
  lifecycleLocked: boolean;
  loadStatus: string;
  microphoneAudioSourceId: string | null;
  pendingDuplicateCaptureSources: DuplicateCaptureSourceCandidate[];
  onSelectAudioSource: (role: AudioSourceRole, sourceId: string | null) => void;
  onDeleteSceneSource: (sceneSourceId: string) => void;
  onResolveDuplicateCaptureSource: (
    resolution: DuplicateCaptureSourceResolution,
  ) => void;
  onSelectCaptureSources: () => void;
  onSelectSceneSource: (sourceId: string | null) => void;
  scene: RecorderScene | null;
  selectedSceneSourceId: string | null;
}

export function CaptureSourcesPanel({
  audioSources,
  desktopAudioSourceId,
  lifecycleLocked,
  loadStatus,
  microphoneAudioSourceId,
  pendingDuplicateCaptureSources,
  onSelectAudioSource,
  onDeleteSceneSource,
  onResolveDuplicateCaptureSource,
  onSelectCaptureSources,
  onSelectSceneSource,
  scene,
  selectedSceneSourceId,
}: CaptureSourcesPanelProps) {
  return (
    <ScrollArea className="h-full">
      <div className="p-4">
        <DuplicateCaptureSourceDialog
          candidates={pendingDuplicateCaptureSources}
          onResolve={onResolveDuplicateCaptureSource}
        />
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-sm font-medium">Sources</h2>
          <Button
            aria-label="Add monitor sources"
            className="gap-2"
            disabled={lifecycleLocked || loadStatus === 'loading'}
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

        <div className="flex items-center justify-between gap-3">
          <h2 className="text-sm font-medium">Audio</h2>
          <AudioSourcePickerDialog
            desktopAudioSourceId={desktopAudioSourceId}
            microphoneAudioSourceId={microphoneAudioSourceId}
            sources={audioSources}
            onSelectAudioSource={onSelectAudioSource}
          >
            <Button
              aria-label="Add audio source"
              className="gap-2"
              disabled={lifecycleLocked}
              size="sm"
              type="button"
              variant="outline"
            >
              <Volume2 className="size-3" />
              Add
            </Button>
          </AudioSourcePickerDialog>
        </div>
        <div className="mt-3 grid gap-3">
          <AudioSourceList
            desktopAudioSourceId={desktopAudioSourceId}
            microphoneAudioSourceId={microphoneAudioSourceId}
            sources={audioSources}
            onSelectAudioSource={onSelectAudioSource}
          />
        </div>
      </div>
    </ScrollArea>
  );
}

interface DuplicateCaptureSourceDialogProps {
  candidates: DuplicateCaptureSourceCandidate[];
  onResolve: (resolution: DuplicateCaptureSourceResolution) => void;
}

function DuplicateCaptureSourceDialog({
  candidates,
  onResolve,
}: DuplicateCaptureSourceDialogProps) {
  const candidate = candidates[0] ?? null;

  return (
    <Dialog open={Boolean(candidate)}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>Duplicate monitor</DialogTitle>
          <DialogDescription>
            {candidate
              ? `${candidate.source.displayName} looks like ${candidate.existingSource.displayName}.`
              : 'This monitor looks like an existing source.'}
          </DialogDescription>
        </DialogHeader>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onResolve('skip')}
          >
            Skip
          </Button>
          <Button
            type="button"
            variant="outline"
            onClick={() => onResolve('add-anyway')}
          >
            Add anyway
          </Button>
          <Button type="button" onClick={() => onResolve('refresh-existing')}>
            Refresh existing
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
