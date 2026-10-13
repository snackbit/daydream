import { memo } from 'react';
import { AppWindow, Keyboard, Mic, Mouse, Volume2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { InspectorContentEntry } from '../inspector-content';
import {
  playbackTimestamp,
  type PlaybackWordSpan,
} from './playback-feed-model';
import { PlaybackObservation } from './playback-observation';
import { useEntrySeek } from './use-entry-seek';

export interface PlaybackFeedEntryProps {
  entry: InspectorContentEntry;
  showSourceHeading: boolean;
  spans: readonly PlaybackWordSpan[];
  activeWordId: string | undefined;
  phase: 'past' | 'current' | 'upcoming';
  onSeek: (time: number) => void;
  onNavigateEntry?: ((entry: InspectorContentEntry) => void) | undefined;
}
export const PlaybackFeedEntry = memo(function PlaybackFeedEntry({
  entry,
  showSourceHeading,
  spans,
  activeWordId,
  phase,
  onSeek,
  onNavigateEntry,
}: PlaybackFeedEntryProps) {
  const seek = useEntrySeek(entry, onSeek, onNavigateEntry);
  if (entry.kind === 'observation')
    return (
      <PlaybackObservation
        entry={entry}
        onSeek={onSeek}
        onNavigateEntry={onNavigateEntry}
      />
    );
  const Icon =
    entry.kind === 'focus'
      ? AppWindow
      : entry.kind === 'activity'
        ? entry.source === 'Mouse'
          ? Mouse
          : Keyboard
        : entry.source === 'Microphone'
          ? Mic
          : Volume2;
  if (entry.kind === 'focus' || entry.kind === 'activity')
    return (
      <div
        data-inspector-entry={entry.id}
        className="text-muted-foreground min-w-0 py-1"
      >
        <Button
          variant="ghost"
          size="sm"
          onClick={seek}
          aria-label={`Jump to ${entry.source}`}
          title={entry.text}
          className="h-auto w-full min-w-0 justify-start gap-1.5 px-0 py-0 text-xs font-normal select-text [&_svg]:size-3"
        >
          <Icon aria-hidden="true" className="size-3 shrink-0" />
          <span className="min-w-0 flex-1 truncate text-left">
            {entry.text}
          </span>
        </Button>
      </div>
    );
  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={seek}
      aria-label={`Jump to transcript from ${entry.source} at ${playbackTimestamp(entry.startMs)}`}
      data-inspector-entry={entry.id}
      data-phase={phase}
      className="data-[phase=past]:text-muted-foreground data-[phase=upcoming]:text-muted-foreground h-auto w-full min-w-0 flex-col items-stretch gap-0.5 rounded-xs px-0 py-1 text-left font-normal whitespace-normal select-text [&_svg]:size-3"
    >
      {showSourceHeading ? (
        <span className="text-muted-foreground flex items-center justify-between gap-2 text-[11px] leading-4">
          <span className="flex min-w-0 items-center gap-1.5">
            <Icon aria-hidden="true" className="size-3 shrink-0" />
            <span className="truncate">{entry.source}</span>
          </span>
        </span>
      ) : null}
      <span className="block text-sm leading-5 wrap-anywhere whitespace-pre-wrap">
        {spans.length
          ? spans.map((span, index) => (
              <span
                key={index}
                data-word-id={span.word?.id}
                aria-current={
                  span.word?.id === activeWordId && activeWordId !== undefined
                    ? 'true'
                    : undefined
                }
                className="aria-[current=true]:bg-annotation-gold/25 aria-[current=true]:text-foreground rounded-xs"
              >
                {span.text}
              </span>
            ))
          : entry.text}
      </span>
    </Button>
  );
});
