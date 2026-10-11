import { useEffect, useRef, useState } from 'react';
import { Sparkles } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import type { InspectorContentEntry } from '../inspector-content';
import { playbackTimestamp } from './playback-feed-model';
import { useEntrySeek } from './use-entry-seek';

export interface PlaybackObservationProps {
  entry: InspectorContentEntry;
  onSeek: (time: number) => void;
  onNavigateEntry?: ((entry: InspectorContentEntry) => void) | undefined;
}
export function PlaybackObservation({
  entry,
  onSeek,
  onNavigateEntry,
}: PlaybackObservationProps) {
  const seek = useEntrySeek(entry, onSeek, onNavigateEntry);
  const [open, setOpen] = useState(false);
  const [overflow, setOverflow] = useState(false);
  const preview = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    if (open) return;
    const element = preview.current;
    if (!element) return;
    const measure = () =>
      setOverflow(element.scrollHeight > element.clientHeight);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [entry.text, open]);
  return (
    <div
      data-inspector-entry={entry.id}
      className="border-annotation-gold/50 my-1 border-l-2 pl-2.5"
    >
      <Collapsible open={open} onOpenChange={setOpen} className="space-y-0.5">
        <div className="text-muted-foreground flex items-center justify-between gap-2 text-[11px] leading-4">
          <Button
            variant="ghost"
            size="sm"
            onClick={seek}
            aria-label={`Jump to annotation from ${entry.source} at ${playbackTimestamp(entry.startMs)}`}
            className="h-auto min-w-0 justify-start gap-1.5 px-0 py-0 text-[11px] leading-4 font-normal select-text [&_svg]:size-3"
          >
            <Sparkles
              className="text-annotation-gold size-3 shrink-0"
              aria-hidden="true"
            />
            <span className="truncate">AI observation · {entry.source}</span>
          </Button>
        </div>
        {!open ? (
          <Button
            variant="ghost"
            size="sm"
            onClick={seek}
            aria-label={`Jump to annotation text from ${entry.source} at ${playbackTimestamp(entry.startMs)}`}
            className="h-auto w-full min-w-0 justify-start rounded-xs px-0 py-0 text-left font-normal whitespace-normal select-text"
          >
            <span
              ref={preview}
              className="line-clamp-2 text-xs leading-4 break-words whitespace-pre-wrap"
            >
              {entry.text}
            </span>
          </Button>
        ) : null}
        <CollapsibleContent className="text-xs leading-4 break-words whitespace-pre-wrap">
          <Button
            variant="ghost"
            size="sm"
            onClick={seek}
            aria-label={`Jump to annotation text from ${entry.source} at ${playbackTimestamp(entry.startMs)}`}
            className="h-auto w-full min-w-0 justify-start rounded-xs px-0 py-0 text-left text-xs leading-4 font-normal whitespace-pre-wrap select-text"
          >
            {entry.text}
          </Button>
        </CollapsibleContent>
        {overflow || open ? (
          <CollapsibleTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              className="text-muted-foreground h-auto px-0 py-0 text-[11px]"
              aria-label={`${open ? 'Collapse' : 'Expand'} AI observation from ${entry.source} at ${playbackTimestamp(entry.startMs)}`}
            >
              {open ? 'Less' : 'More'}
            </Button>
          </CollapsibleTrigger>
        ) : null}
      </Collapsible>
    </div>
  );
}
