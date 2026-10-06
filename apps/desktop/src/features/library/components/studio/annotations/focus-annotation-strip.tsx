import type { MouseEvent } from 'react';
import type {
  ActivityEvent,
  LibraryRecordingRedaction,
  LibraryRecordingSegment,
} from '@daydream/shared';
import { AppWindow } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { StudioInspectorSelection } from '../studio-inspector-selection';
import type { StudioLaneDraft } from '../studio-timeline-store';

export interface FocusAnnotationStripProps {
  events: ActivityEvent[];
  lane: StudioLaneDraft;
  redactions: LibraryRecordingRedaction[];
  segments: LibraryRecordingSegment[];
  timelineWidthPx: number;
  totalDurationMs: number;
  onInspectActivity: (selection: StudioInspectorSelection) => void;
}

export function FocusAnnotationStrip(props: FocusAnnotationStripProps) {
  const points = props.events
    .filter((event) => event.kind === 'focused-window')
    .sort((a, b) => a.offsetMs - b.offsetMs);
  return (
    <div
      className="absolute inset-x-0 top-6 h-6 border-t"
      aria-label="Activity descriptions"
    >
      {points.map((event, index) => {
        const segment = props.segments.find(
          (segment) =>
            event.offsetMs >= segment.playbackOffsetMs &&
            event.offsetMs <
              segment.playbackOffsetMs + (segment.durationMs ?? 0),
        );
        let endMs = Math.min(
          points[index + 1]?.offsetMs ?? props.totalDurationMs,
          segment
            ? segment.playbackOffsetMs + (segment.durationMs ?? 0)
            : event.offsetMs,
        );
        for (const redaction of props.redactions) {
          if (
            event.offsetMs >= redaction.startOffsetMs &&
            event.offsetMs < redaction.endOffsetMs
          )
            return null;
          if (redaction.startOffsetMs > event.offsetMs)
            endMs = Math.min(endMs, redaction.startOffsetMs);
        }
        if (endMs <= event.offsetMs) return null;
        return (
          <FocusDescription
            key={event.id}
            {...props}
            event={event}
            endMs={endMs}
          />
        );
      })}
    </div>
  );
}

function FocusDescription({
  event,
  endMs,
  lane,
  timelineWidthPx,
  totalDurationMs,
  onInspectActivity,
}: FocusAnnotationStripProps & { event: ActivityEvent; endMs: number }) {
  const label =
    [event.applicationName, event.title].filter(Boolean).join(' · ') ||
    'Focused window';
  function inspect(click: MouseEvent<HTMLButtonElement>) {
    click.stopPropagation();
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
    <Button
      variant="ghost"
      type="button"
      aria-label={`Inspect ${label}`}
      title={label}
      data-library-action
      className="absolute h-6 justify-start gap-1 overflow-hidden rounded-none border-r px-1 text-[11px] font-normal"
      style={{
        left: (event.offsetMs / totalDurationMs) * timelineWidthPx,
        width: ((endMs - event.offsetMs) / totalDurationMs) * timelineWidthPx,
      }}
      onClick={inspect}
    >
      <AppWindow aria-hidden="true" className="shrink-0" />
      <span className="truncate">{label}</span>
    </Button>
  );
}
