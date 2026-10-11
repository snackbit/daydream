import type { RefObject } from 'react';
import { ArrowDown, ArrowUp, ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  playbackTimestamp,
  type PlaybackFeedFrame,
} from '../playback-feed-model';
import { usePlaybackRuler } from './use-playback-ruler';

export interface PlaybackRulerProps {
  viewport: HTMLDivElement | null;
  list: RefObject<HTMLOListElement | null>;
  positions: ReadonlyMap<string, number>;
  frame: PlaybackFeedFrame;
  firstStartMs: number | undefined;
  lastEndMs: number | undefined;
  onReturn: () => void;
}
export function PlaybackRuler({
  viewport,
  list,
  positions,
  frame,
  firstStartMs,
  lastEndMs,
  onReturn,
}: PlaybackRulerProps) {
  const wordId = frame.rulerWordId;
  const boundary =
    firstStartMs !== undefined && frame.time < firstStartMs
      ? 'before'
      : lastEndMs !== undefined &&
          frame.time >= lastEndMs &&
          frame.activeEntries.size === 0
        ? 'after'
        : null;
  const position = usePlaybackRuler({
    viewport,
    list,
    positions,
    currentId: frame.currentId,
    wordId,
    boundary,
  });
  const Direction = position?.kind === 'above' ? ArrowUp : ArrowDown;
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden">
      <span
        aria-hidden="true"
        className="bg-muted-foreground/25 absolute inset-y-0 left-[87px] w-px"
      />
      {position?.kind === 'visible' ? (
        <ChevronRight
          aria-label="Current playback position"
          data-entry-id={frame.currentId}
          data-word-id={wordId}
          className="text-annotation-gold absolute left-[79px] size-4 -translate-y-1/2"
          style={{ top: position.top }}
        />
      ) : position ? (
        <Button
          size="sm"
          variant="ghost"
          data-direction={position.kind}
          onClick={onReturn}
          aria-label={`Playback is ${position.kind} at ${playbackTimestamp(frame.time)}; resume auto-scroll`}
          className="bg-muted/95 text-annotation-gold pointer-events-auto absolute left-6 z-10 h-6 w-16 gap-0.5 rounded-full border px-1 text-[11px] tabular-nums data-[direction=above]:top-0 data-[direction=below]:bottom-0"
        >
          <Direction aria-hidden="true" className="size-3 shrink-0" />
          {playbackTimestamp(frame.time)}
        </Button>
      ) : null}
    </div>
  );
}
