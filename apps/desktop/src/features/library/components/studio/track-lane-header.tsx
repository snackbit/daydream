import type { ButtonHTMLAttributes } from 'react';
import type { LibraryRecordingTrack } from '@daydream/shared';
import { GripVertical, Volume2, VolumeX } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { studioColorById, type StudioLaneDraft } from './studio-timeline-store';
import { TRACK_LANE_HEIGHT_PX } from './track-timeline-math';

export function TrackLaneHeader({
  audioTrack,
  dragHandleProps,
  lane,
  muted,
  onSelectLane,
  onToggleTrackMuted,
  selectedLane,
  track,
}: {
  audioTrack: boolean;
  dragHandleProps: ButtonHTMLAttributes<HTMLButtonElement>;
  lane: StudioLaneDraft;
  muted: boolean;
  onSelectLane: (trackId: string, additive: boolean) => void;
  onToggleTrackMuted: (trackId: string) => void;
  selectedLane: boolean;
  track: LibraryRecordingTrack;
}) {
  const color = studioColorById(lane.colorId);

  return (
    <div
      className={cn(
        'bg-background group-hover/lane:bg-muted/70 sticky left-0 z-10 flex min-w-0 flex-col border-r border-b transition-colors',
        selectedLane && 'bg-accent text-accent-foreground',
      )}
      data-track-id={track.id}
      style={{ height: TRACK_LANE_HEIGHT_PX }}
    >
      <div className="flex min-h-0 flex-1 items-center gap-1 p-2">
        <Button
          aria-label={`Reorder ${lane.label}`}
          className="cursor-grab active:cursor-grabbing"
          data-library-action
          size="icon-xs"
          type="button"
          variant="ghost"
          {...dragHandleProps}
        >
          <GripVertical className="size-3" />
        </Button>
        {audioTrack ? (
          <Button
            aria-label={muted ? `Unmute ${lane.label}` : `Mute ${lane.label}`}
            data-library-action
            size="icon"
            variant={muted ? 'outline' : 'ghost'}
            onClick={() => onToggleTrackMuted(track.id)}
          >
            {muted ? (
              <VolumeX className="size-3.5" />
            ) : (
              <Volume2 className="size-3.5" />
            )}
          </Button>
        ) : null}
        <button
          aria-label={`Select ${lane.label}`}
          aria-pressed={selectedLane}
          className="hover:bg-muted focus-visible:ring-ring/30 min-w-0 flex-1 rounded-sm px-1 py-0.5 text-left transition-colors outline-none focus-visible:ring-2"
          data-library-action
          type="button"
          onClick={(event) =>
            onSelectLane(
              track.id,
              event.shiftKey || event.metaKey || event.ctrlKey,
            )
          }
        >
          <span className="flex items-center gap-2">
            <span
              aria-hidden="true"
              className="size-2.5 shrink-0 rounded-full"
              style={{ backgroundColor: color.strong }}
            />
            <span className="min-w-0">
              <span className="block truncate text-xs font-medium">
                {lane.label}
              </span>
              <span className="text-muted-foreground block truncate text-xs">
                {lane.description}
              </span>
            </span>
          </span>
        </button>
      </div>
    </div>
  );
}
