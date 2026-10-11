import { useCallback } from 'react';
import { Button } from '@/components/ui/button';
import type { InspectorContentEntry } from '../../inspector-content';
import { playbackTimestamp } from '../playback-feed-model';

export interface PlaybackTimeTickProps {
  entry: InspectorContentEntry;
  onSeek: (time: number) => void;
  onNavigateEntry?: ((entry: InspectorContentEntry) => void) | undefined;
}
export function PlaybackTimeTick({
  entry,
  onSeek,
  onNavigateEntry,
}: PlaybackTimeTickProps) {
  const seek = useCallback(() => {
    if (onNavigateEntry) onNavigateEntry(entry);
    else onSeek(entry.startMs);
  }, [entry, onSeek, onNavigateEntry]);
  const source =
    entry.kind === 'observation'
      ? `AI observation from ${entry.source}`
      : entry.source;
  return (
    <div className="relative w-16 shrink-0 pt-1">
      <Button
        variant="ghost"
        size="sm"
        onClick={seek}
        aria-label={`Jump to ${source} at ${playbackTimestamp(entry.startMs)}`}
        className="text-muted-foreground h-auto w-full justify-end rounded-none py-0 pr-3 pl-0 text-[11px] leading-4 font-normal tabular-nums"
      >
        {playbackTimestamp(entry.startMs)}
      </Button>
      <span
        aria-hidden="true"
        className="bg-muted-foreground/25 pointer-events-none absolute top-3 right-0 h-px w-1.5"
      />
    </div>
  );
}
