import type { ReactNode } from 'react';
import { useMemo, useState } from 'react';
import type { AudioSource, AudioSourceRole } from '@daydream/shared';
import { Mic, MonitorSpeaker, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

const audioRoleLabels = {
  'desktop-audio': 'Desktop',
  microphone: 'Microphone',
} satisfies Record<AudioSourceRole, string>;

export interface AudioSourceListProps {
  desktopAudioSourceId: string | null;
  disabled?: boolean;
  microphoneAudioSourceId: string | null;
  onSelectAudioSource: (role: AudioSourceRole, sourceId: string | null) => void;
  sources: AudioSource[];
}

export function AudioSourceList({
  desktopAudioSourceId,
  disabled = false,
  microphoneAudioSourceId,
  onSelectAudioSource,
  sources,
}: AudioSourceListProps) {
  const selectedSources = useMemo(
    () =>
      (
        [
          {
            role: 'desktop-audio',
            sourceId: desktopAudioSourceId,
          },
          {
            role: 'microphone',
            sourceId: microphoneAudioSourceId,
          },
        ] satisfies { role: AudioSourceRole; sourceId: string | null }[]
      ).flatMap(({ role, sourceId }) => {
        const source = sources.find(
          (candidate) =>
            candidate.id === sourceId && candidate.roles.includes(role),
        );

        return source ? [{ role, source }] : [];
      }),
    [desktopAudioSourceId, microphoneAudioSourceId, sources],
  );

  if (selectedSources.length === 0) {
    return (
      <p className="text-muted-foreground border-y py-2 text-xs">
        No audio sources selected.
      </p>
    );
  }

  return (
    <div className="divide-y border-y">
      {selectedSources.map(({ role, source }) => {
        const handleRemove = () => onSelectAudioSource(role, null);

        return (
          <div className="flex items-center gap-3 px-3 py-3" key={role}>
            <AudioRoleIcon role={role} />
            <div className="min-w-0 flex-1">
              <div className="text-sm font-medium">{audioRoleLabels[role]}</div>
              <div className="text-muted-foreground text-xs break-words">
                {source.displayName}
              </div>
            </div>
            <Button
              aria-label={`Delete ${audioRoleLabels[role]} audio`}
              disabled={disabled}
              onClick={handleRemove}
              size="icon"
              type="button"
              variant="ghost"
            >
              <Trash2 className="size-3" />
            </Button>
          </div>
        );
      })}
    </div>
  );
}

export interface AudioSourcePickerDialogProps extends AudioSourceListProps {
  children: ReactNode;
}

export function AudioSourcePickerDialog({
  children,
  desktopAudioSourceId,
  disabled = false,
  microphoneAudioSourceId,
  onSelectAudioSource,
  sources,
}: AudioSourcePickerDialogProps) {
  const [open, setOpen] = useState(false);
  const eligibleOptions = useMemo(
    () =>
      buildEligibleAudioOptions({
        desktopAudioSourceId,
        microphoneAudioSourceId,
        sources,
      }),
    [desktopAudioSourceId, microphoneAudioSourceId, sources],
  );
  const [selectedOptionValue, setSelectedOptionValue] = useState('');
  const defaultOptionValue = eligibleOptions[0]?.value ?? '';
  const activeOptionValue = eligibleOptions.some(
    (option) => option.value === selectedOptionValue,
  )
    ? selectedOptionValue
    : defaultOptionValue;
  const selectedOption =
    eligibleOptions.find((option) => option.value === activeOptionValue) ??
    null;

  return (
    <Dialog
      open={disabled ? false : open}
      onOpenChange={(nextOpen) => {
        if (disabled) {
          setOpen(false);
          return;
        }

        if (nextOpen) {
          setSelectedOptionValue(defaultOptionValue);
        }

        setOpen(nextOpen);
      }}
    >
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add audio source</DialogTitle>
          <DialogDescription>
            Select an audio device to include in this recording.
          </DialogDescription>
        </DialogHeader>

        {eligibleOptions.length > 0 ? (
          <Select
            onValueChange={setSelectedOptionValue}
            value={activeOptionValue}
          >
            <SelectTrigger aria-label="Audio source" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {eligibleOptions.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        ) : (
          <div className="text-muted-foreground rounded-md border px-3 py-2 text-xs">
            No more devices available
          </div>
        )}

        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" variant="outline">
              Cancel
            </Button>
          </DialogClose>
          <Button
            disabled={!selectedOption}
            type="button"
            onClick={() => {
              if (!selectedOption) {
                return;
              }

              onSelectAudioSource(
                selectedOption.role,
                selectedOption.source.id,
              );
              setOpen(false);
            }}
          >
            Add
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface EligibleAudioOptionsInput {
  desktopAudioSourceId: string | null;
  microphoneAudioSourceId: string | null;
  sources: AudioSource[];
}

interface EligibleAudioOption {
  label: string;
  role: AudioSourceRole;
  source: AudioSource;
  value: string;
}

function buildEligibleAudioOptions({
  desktopAudioSourceId,
  microphoneAudioSourceId,
  sources,
}: EligibleAudioOptionsInput): EligibleAudioOption[] {
  const selectedSourceIds = new Set(
    [desktopAudioSourceId, microphoneAudioSourceId].filter(
      (sourceId): sourceId is string => Boolean(sourceId),
    ),
  );

  return sources
    .filter((source) => !selectedSourceIds.has(source.id))
    .map((source) => {
      const role = preferredAudioRoleForSource({
        desktopAudioSourceId,
        microphoneAudioSourceId,
        source,
      });

      return {
        label: `${source.displayName}${source.isDefault ? ' (default)' : ''}`,
        role,
        source,
        value: source.id,
      };
    });
}

function preferredAudioRoleForSource({
  desktopAudioSourceId,
  microphoneAudioSourceId,
  source,
}: {
  desktopAudioSourceId: string | null;
  microphoneAudioSourceId: string | null;
  source: AudioSource;
}): AudioSourceRole {
  if (source.roles.includes('desktop-audio') && !desktopAudioSourceId) {
    return 'desktop-audio';
  }

  if (source.roles.includes('microphone') && !microphoneAudioSourceId) {
    return 'microphone';
  }

  return source.roles[0] ?? 'microphone';
}

function AudioRoleIcon({ role }: { role: AudioSourceRole }) {
  if (role === 'desktop-audio') {
    return <MonitorSpeaker className="text-muted-foreground size-3" />;
  }

  return <Mic className="text-muted-foreground size-3" />;
}
