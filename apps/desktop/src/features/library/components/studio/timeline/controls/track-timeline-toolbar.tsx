import { type ReactNode } from 'react';
import { Minus, Plus, ShieldOff, ShieldX } from 'lucide-react';
import { Button } from '@/components/ui/button';

export interface TrackTimelineToolbarProps {
  canZoomIn: boolean;
  canZoomOut: boolean;
  hasTimeRangeSelection: boolean;
  onRedactSelection: () => void;
  onUnredactSelection: () => void;
  redactionPending: boolean;
  onZoomIn: () => void;
  onZoomOut: () => void;
  selectedRangeRedactionCount: number;
  statusText: string;
  zoomPercent: number;
  selectionActions?: ReactNode;
  followAction?: ReactNode;
}

export function TrackTimelineToolbar({
  canZoomIn,
  canZoomOut,
  hasTimeRangeSelection,
  onRedactSelection,
  onUnredactSelection,
  redactionPending,
  onZoomIn,
  onZoomOut,
  selectedRangeRedactionCount,
  statusText,
  zoomPercent,
  selectionActions,
  followAction,
}: TrackTimelineToolbarProps) {
  return (
    <div
      data-studio-timeline-toolbar
      className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-b px-6 py-4"
    >
      <div className="text-sm font-medium">Tracks</div>
      <div className="flex min-w-0 flex-wrap items-center gap-3">
        {followAction}
        {selectionActions}
        {hasTimeRangeSelection ? (
          <div className="flex shrink-0 items-center gap-1">
            <Button
              disabled={redactionPending}
              size="sm"
              type="button"
              variant="outline"
              onClick={onRedactSelection}
            >
              <ShieldX data-icon="inline-start" />
              Redact
            </Button>
            {selectedRangeRedactionCount > 0 ? (
              <Button
                disabled={redactionPending}
                size="sm"
                type="button"
                variant="outline"
                onClick={onUnredactSelection}
              >
                <ShieldOff data-icon="inline-start" />
                Unredact
              </Button>
            ) : null}
          </div>
        ) : null}
        <div className="text-muted-foreground text-xs">{statusText}</div>
        <div
          aria-label="Track zoom"
          className="flex shrink-0 items-center gap-1"
        >
          <Button
            aria-label="Zoom out tracks"
            disabled={!canZoomOut}
            size="icon"
            type="button"
            variant="ghost"
            onClick={onZoomOut}
          >
            <Minus className="size-3" />
          </Button>
          <div
            aria-live="polite"
            className="text-muted-foreground w-12 text-center text-xs tabular-nums"
          >
            {zoomPercent}%
          </div>
          <Button
            aria-label="Zoom in tracks"
            disabled={!canZoomIn}
            size="icon"
            type="button"
            variant="ghost"
            onClick={onZoomIn}
          >
            <Plus className="size-3" />
          </Button>
        </div>
      </div>
    </div>
  );
}
