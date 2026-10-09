import type { ButtonHTMLAttributes } from 'react';
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
  heightPx = TRACK_LANE_HEIGHT_PX,
  onSelectLane,
  onToggleTrackMuted,
  selectedLane,
  trackId,
}: {
  audioTrack: boolean;
  dragHandleProps: ButtonHTMLAttributes<HTMLButtonElement>;
  lane: StudioLaneDraft;
  heightPx?: number;
  muted: boolean;
  onSelectLane: (trackId: string, additive: boolean) => void;
  onToggleTrackMuted: (trackId: string) => void;
  selectedLane: boolean;
  trackId: string;
}) {
  const color = studioColorById(lane.colorId);
  const compact = heightPx < TRACK_LANE_HEIGHT_PX;

  return (
    <div
      className={cn(
        'bg-background group-hover/lane:bg-muted sticky left-0 z-50 flex min-w-0 flex-col border-r border-b transition-colors',
        selectedLane && 'bg-accent text-accent-foreground',
      )}
      data-track-id={trackId}
      style={{ height: heightPx }}
    >
      <div
        className={cn(
          'flex min-h-0 flex-1 items-center',
          compact ? 'gap-0.5 px-1 py-0' : 'gap-1 p-2',
        )}
      >
        <Button
          aria-label={`Reorder ${lane.label}`}
          className={cn(
            'cursor-grab active:cursor-grabbing',
            compact && 'size-5',
          )}
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
            onClick={() => onToggleTrackMuted(trackId)}
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
          className="hover:bg-muted focus-visible:ring-ring min-w-0 flex-1 rounded-sm px-1 py-0.5 text-left transition-colors outline-none focus-visible:ring-2"
          data-library-action
          type="button"
          onClick={(event) =>
            onSelectLane(
              trackId,
              event.shiftKey || event.metaKey || event.ctrlKey,
            )
          }
        >
          <span className="flex items-center gap-2">
            <span
              aria-hidden="true"
              className={cn(
                'shrink-0 rounded-full',
                compact ? 'size-2' : 'size-2.5',
              )}
              style={{
                backgroundColor: `color-mix(in oklch, ${color.strong} 42%, var(--muted-foreground))`,
              }}
            />
            <span className="min-w-0">
              <span
                className={cn(
                  'block truncate text-xs font-medium',
                  compact && 'leading-none',
                )}
              >
                {lane.label}
              </span>
              {compact ? null : (
                <span className="text-muted-foreground block truncate text-xs">
                  {lane.description}
                </span>
              )}
            </span>
          </span>
        </button>
      </div>
    </div>
  );
}
