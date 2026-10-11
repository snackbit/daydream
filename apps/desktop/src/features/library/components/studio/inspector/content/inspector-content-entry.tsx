import { useCallback } from 'react';
import {
  Keyboard,
  Mic,
  Monitor,
  Mouse,
  Volume2,
  AppWindow,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { formatTimelineOffset } from '../inspector-format';
import type { InspectorContentEntry } from './inspector-content';

export interface InspectorContentEntryCardProps {
  entry: InspectorContentEntry;
  onSeek?: ((time: number) => void) | undefined;
  phase?: 'past' | 'current' | 'upcoming' | undefined;
  current?: boolean | undefined;
}
export function InspectorContentEntryCard({
  entry,
  onSeek,
  phase,
  current,
}: InspectorContentEntryCardProps) {
  const seek = useCallback(
    () => onSeek?.(entry.startMs),
    [entry.startMs, onSeek],
  );
  const Icon =
    entry.kind === 'focus'
      ? AppWindow
      : entry.kind === 'activity'
        ? entry.source === 'Mouse'
          ? Mouse
          : Keyboard
        : entry.kind !== 'speech'
          ? Monitor
          : entry.source === 'Microphone'
            ? Mic
            : Volume2;
  return (
    <li
      aria-current={current ? 'true' : undefined}
      data-inspector-entry={entry.id}
      data-phase={phase}
      className="data-[phase=current]:border-border data-[phase=current]:bg-background/70 data-[phase=past]:text-muted-foreground data-[phase=upcoming]:text-muted-foreground space-y-2 rounded-lg border border-transparent px-3 py-3 data-[phase=past]:opacity-70 data-[phase=upcoming]:opacity-50"
    >
      <div className="flex items-center justify-between gap-2 text-xs">
        <span className="flex min-w-0 items-center gap-2 font-medium">
          <Icon aria-hidden="true" className="size-3.5 shrink-0" />
          {entry.source}
          {entry.kind === 'observation'
            ? ' · Observation'
            : entry.kind === 'screen-text'
              ? ' · Screen text'
              : null}
        </span>
        {onSeek ? (
          <Button
            aria-label={`Jump to ${entry.source} at ${formatTimelineOffset(entry.startMs)}`}
            variant="ghost"
            size="sm"
            className="h-auto shrink-0 px-1 py-0 text-xs tabular-nums"
            onClick={seek}
          >
            {formatTimelineOffset(entry.startMs)}
          </Button>
        ) : (
          <span className="tabular-nums">
            {formatTimelineOffset(entry.startMs)}
          </span>
        )}
      </div>
      <p className="text-sm leading-6 break-words whitespace-pre-wrap">
        {entry.fragments.length
          ? entry.fragments.map((fragment, i) => (
              <span
                key={i}
                data-emphasis={fragment.emphasis}
                className="data-[emphasis=context]:text-muted-foreground data-[emphasis=selected]:text-foreground"
              >
                {fragment.text}
              </span>
            ))
          : entry.text}
      </p>
      {entry.boundary &&
      (entry.wordTimingUnavailable ?? entry.words.length === 0) ? (
        <p className="text-muted-foreground text-xs">
          Partly selected · {formatTimelineOffset(entry.startMs)}–
          {formatTimelineOffset(entry.endMs)}. Exact word timing is unavailable.
        </p>
      ) : null}
    </li>
  );
}
