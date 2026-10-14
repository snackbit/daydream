import {
  useRef,
  useState,
  type MouseEvent,
  type PointerEvent,
  type KeyboardEvent,
} from 'react';
import type { LibraryRecordingRedaction } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { resizeLibraryRedaction } from '../../../library-client';
import type { TimelineLaneLayout } from '../timeline/interactions/track-timeline-math';
import {
  TRACK_LABEL_WIDTH_PX,
  TRACK_RULER_HEIGHT_PX,
} from '../timeline/interactions/track-timeline-math';

export interface GroupedRedactionsProps {
  redactions: LibraryRecordingRedaction[];
  lanes: TimelineLaneLayout[];
  selectedIds: Set<string>;
  timelineWidthPx: number;
  totalDurationMs: number;
  onSelect: (id: string, additive: boolean) => void;
  onSaved: () => void;
  onResize?: typeof resizeLibraryRedaction | undefined;
}

export function GroupedRedactions(props: GroupedRedactionsProps) {
  return props.redactions.flatMap((redaction) => {
    const groups: { top: number; height: number; labels: string[] }[] = [];
    let top = TRACK_RULER_HEIGHT_PX;
    let previousTarget = false;
    for (const lane of props.lanes) {
      const target = redaction.trackIds.includes(lane.id);
      const previous = groups[groups.length - 1];
      if (target && previousTarget && previous) {
        previous.height += lane.heightPx;
        previous.labels.push(lane.label ?? 'Track');
      } else if (target)
        groups.push({
          top,
          height: lane.heightPx,
          labels: [lane.label ?? 'Track'],
        });
      top += lane.heightPx;
      previousTarget = target;
    }
    return groups.map((group) => (
      <RedactionGroup
        key={`${redaction.id}:${group.top}:${redaction.startOffsetMs}:${redaction.endOffsetMs}`}
        {...props}
        redaction={redaction}
        group={group}
      />
    ));
  });
}

function RedactionGroup({
  redaction,
  group,
  selectedIds,
  timelineWidthPx,
  totalDurationMs,
  onSelect,
  onSaved,
  onResize = resizeLibraryRedaction,
}: GroupedRedactionsProps & {
  redaction: LibraryRecordingRedaction;
  group: { top: number; height: number; labels: string[] };
}) {
  const [bounds, setBounds] = useState({
    start: redaction.startOffsetMs,
    end: redaction.endOffsetMs,
  });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const drag = useRef<{
    edge: 'start' | 'end';
    x: number;
    start: number;
    end: number;
  } | null>(null);
  async function save(start: number, end: number) {
    if (start === redaction.startOffsetMs && end === redaction.endOffsetMs)
      return;
    setSaving(true);
    setError(null);
    try {
      await onResize({
        recordingId: redaction.recordingId,
        redactionId: redaction.id,
        startOffsetMs: start,
        endOffsetMs: end,
        expectedStartOffsetMs: redaction.startOffsetMs,
        expectedEndOffsetMs: redaction.endOffsetMs,
      });
      onSaved();
    } catch {
      setBounds({ start: redaction.startOffsetMs, end: redaction.endOffsetMs });
      setError('The redaction could not be resized. Refresh and try again.');
    } finally {
      setSaving(false);
    }
  }
  function select(event: MouseEvent<HTMLButtonElement>) {
    event.stopPropagation();
    onSelect(redaction.id, event.shiftKey || event.ctrlKey || event.metaKey);
  }
  function down(event: PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0 || saving) return;
    event.stopPropagation();
    event.preventDefault();
    const edge = event.currentTarget.dataset.edge === 'start' ? 'start' : 'end';
    drag.current = {
      edge,
      x: event.clientX,
      start: bounds.start,
      end: bounds.end,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
  }
  function move(event: PointerEvent<HTMLButtonElement>) {
    const current = drag.current;
    if (!current) return;
    const delta = Math.round(
      ((event.clientX - current.x) / timelineWidthPx) * totalDurationMs,
    );
    setBounds(
      current.edge === 'start'
        ? {
            start: Math.max(
              0,
              Math.min(current.end - 1, current.start + delta),
            ),
            end: current.end,
          }
        : {
            start: current.start,
            end: Math.min(
              totalDurationMs,
              Math.max(current.start + 1, current.end + delta),
            ),
          },
    );
  }
  function up(event: PointerEvent<HTMLButtonElement>) {
    event.stopPropagation();
    if (!drag.current) return;
    drag.current = null;
    void save(bounds.start, bounds.end);
  }
  function cancel() {
    drag.current = null;
    setBounds({ start: redaction.startOffsetMs, end: redaction.endOffsetMs });
  }
  function key(event: KeyboardEvent<HTMLButtonElement>) {
    if (saving || !['ArrowLeft', 'ArrowRight'].includes(event.key)) return;
    event.preventDefault();
    event.stopPropagation();
    const delta =
      (event.key === 'ArrowLeft' ? -1 : 1) * (event.shiftKey ? 1000 : 100);
    const next =
      event.currentTarget.dataset.edge === 'start'
        ? {
            start: Math.max(0, Math.min(bounds.end - 1, bounds.start + delta)),
            end: bounds.end,
          }
        : {
            start: bounds.start,
            end: Math.min(
              totalDurationMs,
              Math.max(bounds.start + 1, bounds.end + delta),
            ),
          };
    setBounds(next);
    void save(next.start, next.end);
  }
  return (
    <div
      className="border-muted-foreground/50 data-[selected=true]:border-primary data-[selected=true]:ring-primary/30 absolute z-40 border-x data-[selected=true]:ring-1"
      data-library-action
      data-selected={selectedIds.has(redaction.id)}
      style={{
        top: group.top,
        height: group.height,
        left:
          TRACK_LABEL_WIDTH_PX +
          (bounds.start / totalDurationMs) * timelineWidthPx,
        width: Math.max(
          4,
          ((bounds.end - bounds.start) / totalDurationMs) * timelineWidthPx,
        ),
      }}
    >
      <Button
        aria-label={`${group.labels[0] ?? 'Track'} redacted range`}
        aria-description={`Hidden tracks: ${group.labels.join(', ')}`}
        aria-pressed={selectedIds.has(redaction.id)}
        variant="ghost"
        className="relative h-full w-full items-start justify-start overflow-hidden rounded-none p-0 text-xs hover:bg-transparent dark:hover:bg-transparent"
        data-state={selectedIds.has(redaction.id) ? 'selected' : 'idle'}
        disabled={saving}
        onClick={select}
      >
        <span className="bg-muted text-foreground absolute top-2 left-2 max-w-[calc(100%-1rem)] truncate rounded-sm border px-2 py-0.5 text-xs font-medium">
          {saving ? 'Saving…' : 'Redacted'}
        </span>
        <span className="sr-only">
          {bounds.start} to {bounds.end} ms
        </span>
      </Button>
      <Button
        aria-label="Resize redaction start"
        data-edge="start"
        variant="ghost"
        className="hover:bg-primary/15 focus-visible:bg-primary/20 absolute inset-y-0 left-0 h-full w-1.5 cursor-ew-resize rounded-none p-0"
        disabled={saving}
        onPointerDown={down}
        onPointerMove={move}
        onPointerUp={up}
        onPointerCancel={cancel}
        onKeyDown={key}
      />
      <Button
        aria-label="Resize redaction end"
        data-edge="end"
        variant="ghost"
        className="hover:bg-primary/15 focus-visible:bg-primary/20 absolute inset-y-0 right-0 h-full w-1.5 cursor-ew-resize rounded-none p-0"
        disabled={saving}
        onPointerDown={down}
        onPointerMove={move}
        onPointerUp={up}
        onPointerCancel={cancel}
        onKeyDown={key}
      />
      {error ? (
        <div
          role="alert"
          className="bg-background text-destructive absolute top-full p-1 text-xs"
        >
          {error}
        </div>
      ) : null}
    </div>
  );
}
