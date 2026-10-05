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
  configurationLocked: boolean;
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
  configurationLocked,
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
      <div className="p-2.5">
        <DuplicateCaptureSourceDialog
          candidates={configurationLocked ? [] : pendingDuplicateCaptureSources}
          onResolve={onResolveDuplicateCaptureSource}
        />
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-xs font-semibold">Sources</h2>
          <Button
            aria-label="Add monitor sources"
            className="gap-2"
            disabled={
              configurationLocked || lifecycleLocked || loadStatus === 'loading'
            }
            onClick={onSelectCaptureSources}
            size="sm"
            variant="outline"
          >
            <Video className="size-3" />
            Add
          </Button>
        </div>

        <div className="mt-2">
          <SceneSourceList
            deleteDisabled={configurationLocked}
            selectedSceneSourceId={selectedSceneSourceId}
            sources={scene?.sources ?? []}
            onDelete={onDeleteSceneSource}
            onSelect={onSelectSceneSource}
          />
        </div>

        <Separator className="my-3" />

        <div className="flex items-center justify-between gap-3">
          <h2 className="text-xs font-semibold">Audio</h2>
          <AudioSourcePickerDialog
            desktopAudioSourceId={desktopAudioSourceId}
            disabled={configurationLocked}
            microphoneAudioSourceId={microphoneAudioSourceId}
            sources={audioSources}
            onSelectAudioSource={onSelectAudioSource}
          >
            <Button
              aria-label="Add audio source"
              className="gap-2"
              disabled={configurationLocked || lifecycleLocked}
              size="sm"
              type="button"
              variant="outline"
            >
              <Volume2 className="size-3" />
              Add
            </Button>
          </AudioSourcePickerDialog>
        </div>
        <div className="mt-2">
          <AudioSourceList
            desktopAudioSourceId={desktopAudioSourceId}
            disabled={configurationLocked}
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
              ? `${candidate.source.displayName} looks like ${candidate.existingSource.displayName}. Duplicate portal monitor streams are not supported yet.`
              : 'This monitor looks like an existing source. Duplicate portal monitor streams are not supported yet.'}
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
          <Button type="button" onClick={() => onResolve('refresh-existing')}>
            Refresh existing
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
