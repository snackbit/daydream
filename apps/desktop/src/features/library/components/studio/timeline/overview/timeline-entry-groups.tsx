import { useCallback, type MouseEvent, type ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { formatDuration } from '../../../../library-format';

export interface TimelineInterval {
  id: string;
  startMs: number;
  endMs: number;
}
export interface TimelineEntryGroup<T> {
  id: string;
  startMs: number;
  endMs: number;
  items: T[];
}
/** Grid anchored to time zero makes groups stable when the viewport pans. */
export function groupTimelineEntries<T extends TimelineInterval>(
  items: readonly T[],
  pixelsPerMs: number,
): TimelineEntryGroup<T>[] {
  const buckets = new Map<number, TimelineEntryGroup<T>>();
  const groups: TimelineEntryGroup<T>[] = [];
  for (const item of items) {
    if ((item.endMs - item.startMs) * pixelsPerMs >= 24) {
      groups.push({
        id: item.id,
        startMs: item.startMs,
        endMs: item.endMs,
        items: [item],
      });
      continue;
    }
    const bucket = Math.floor((item.startMs * pixelsPerMs) / 48);
    const group = buckets.get(bucket);
    if (group) {
      group.items.push(item);
      group.startMs = Math.min(group.startMs, item.startMs);
      group.endMs = Math.max(group.endMs, item.endMs);
    } else {
      const next = {
        id: `group:${bucket}`,
        startMs: item.startMs,
        endMs: item.endMs,
        items: [item],
      };
      buckets.set(bucket, next);
      groups.push(next);
    }
  }
  return groups.sort((a, b) => a.startMs - b.startMs);
}

export interface TimelineEntryGroupButtonProps {
  group: TimelineEntryGroup<unknown>;
  label: string;
  pixelsPerMs: number;
  onZoom: (start: number, end: number) => void;
  count?: number;
  offsetPx?: number;
  tooltipContent?: ReactNode;
}
export function TimelineEntryGroupButton({
  group,
  label,
  pixelsPerMs,
  onZoom,
  count = group.items.length,
  offsetPx = 0,
  tooltipContent,
}: TimelineEntryGroupButtonProps) {
  const zoom = useCallback(
    (event: MouseEvent) => {
      event.stopPropagation();
      onZoom(group.startMs, group.endMs);
    },
    [group.startMs, group.endMs, onZoom],
  );
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="secondary"
          size="sm"
          data-library-action
          aria-label={`Zoom into ${count} ${label}`}
          onClick={zoom}
          className="pointer-events-auto absolute inset-y-1 h-auto min-h-0 rounded-sm px-1 text-xs tabular-nums"
          style={{
            left: Math.max(
              0,
              Math.floor((group.startMs * pixelsPerMs) / 48) * 48 - offsetPx,
            ),
            width: 44,
          }}
        >
          {count}
        </Button>
      </TooltipTrigger>
      <TooltipContent className="block whitespace-normal">
        {tooltipContent ?? (
          <span>
            {count} {label} · {formatDuration(group.startMs)}–
            {formatDuration(group.endMs)} · Click to zoom
          </span>
        )}
      </TooltipContent>
    </Tooltip>
  );
}
