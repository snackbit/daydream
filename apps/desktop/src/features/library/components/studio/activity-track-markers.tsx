import type { MouseEvent } from 'react';
import type { ActivityInterval, ActivityEvent } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { formatDuration } from '../../library-format';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import { studioColorById, type StudioLaneDraft } from './studio-timeline-store';

export function ActivityIntervalSpan({
  interval,
  lane,
  onInspect,
  timelineWidthPx,
  totalDurationMs,
}: {
  interval: ActivityInterval;
  lane: StudioLaneDraft;
  onInspect: (interval: ActivityInterval) => void;
  timelineWidthPx: number;
  totalDurationMs: number;
}) {
  const color = studioColorById(lane.colorId);
  const startMs = Math.max(0, interval.startOffsetMs);
  const endMs = Math.max(startMs + 1, interval.endOffsetMs);
  const left =
    (Math.min(startMs, totalDurationMs) / totalDurationMs) * timelineWidthPx;
  const width = Math.max(
    4,
    ((Math.min(endMs, totalDurationMs) - Math.min(startMs, totalDurationMs)) /
      totalDurationMs) *
      timelineWidthPx,
  );

  const accessibleName = `${lane.label} activity, ${interval.state}, ${formatDuration(startMs)} to ${formatDuration(endMs)}`;
  const tooltipLabel = `${lane.description} · ${interval.state} · ${formatDuration(startMs)}–${formatDuration(endMs)} · score ${interval.score ?? 'unavailable'}`;

  function handleInspect() {
    onInspect(interval);
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          aria-label={accessibleName}
          className={cn(
            'absolute top-1/2 z-10 h-2 min-w-1 -translate-y-1/2 rounded-full border px-0 hover:brightness-125',
            'focus-visible:ring-ring focus-visible:ring-2',
          )}
          data-activity-span-id={interval.id}
          data-library-action
          size="icon-xs"
          style={{
            backgroundColor: `color-mix(in oklch, ${color.strong} 38%, transparent)`,
            borderColor: `color-mix(in oklch, ${color.border} 48%, var(--border))`,
            left,
            width,
          }}
          title={tooltipLabel}
          type="button"
          variant="ghost"
          onClick={handleInspect}
        />
      </TooltipTrigger>
      <TooltipContent>{tooltipLabel}</TooltipContent>
    </Tooltip>
  );
}

export function FocusActivityMarker({
  event,
  lane,
  onInspectActivity,
  timelineWidthPx,
  totalDurationMs,
}: {
  event: ActivityEvent;
  lane: StudioLaneDraft;
  onInspectActivity: (selection: StudioInspectorSelection) => void;
  timelineWidthPx: number;
  totalDurationMs: number;
}) {
  const left =
    (Math.min(Math.max(0, event.offsetMs), totalDurationMs) / totalDurationMs) *
    timelineWidthPx;
  const title = focusActivityTitle(event);
  const color = studioColorById('rose');

  function handleSelect(clickEvent: MouseEvent<HTMLButtonElement>) {
    clickEvent.stopPropagation();
    onInspectActivity({
      applicationName: event.applicationName,
      eventKind: event.kind,
      kind: 'activity-event',
      occurredAtMs: event.occurredAtMs,
      offsetMs: event.offsetMs,
      timelineOffsetMs: event.offsetMs,
      title: event.title,
      trackId: lane.trackId,
      trackLabel: lane.label,
    });
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          aria-label={title}
          className="bg-muted focus-visible:ring-ring absolute top-1/2 z-20 h-4 w-3.5 -translate-x-1/2 -translate-y-1/2 rounded-sm border px-0 outline-none focus-visible:ring-2"
          data-library-action
          style={{
            backgroundColor: `color-mix(in oklch, ${color.strong} 38%, var(--card))`,
            borderColor: `color-mix(in oklch, ${color.border} 48%, var(--border))`,
            left,
          }}
          title={title}
          type="button"
          variant="ghost"
          onClick={handleSelect}
        >
          <span className="sr-only">{title}</span>
        </Button>
      </TooltipTrigger>
      <TooltipContent>{title}</TooltipContent>
    </Tooltip>
  );
}

function focusActivityTitle(event: ActivityEvent): string {
  const parts = [
    `Focus changed at ${formatDuration(event.offsetMs)}`,
    event.applicationName,
    event.title,
  ].filter(Boolean);

  return parts.join(' - ');
}
