import {
  memo,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
  type WheelEvent,
} from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { InspectorContentEntry } from '../inspector-content';
import { PlaybackFeedEntry } from './playback-feed-entry';
import {
  playbackSpeechStartsGroup,
  type PlaybackFeedFrame,
  type PlaybackFeedIndex,
  type PlaybackWordSpan,
} from './playback-feed-model';
import { PlaybackRuler } from './ruler/playback-ruler';
import { PlaybackTimeTick } from './ruler/playback-time-tick';

export interface PlaybackFeedListProps {
  index: PlaybackFeedIndex;
  frame: PlaybackFeedFrame;
  liveFrame: PlaybackFeedFrame;
  browsing: boolean;
  onBrowse: () => void;
  onSeek: (time: number) => void;
  onNavigateEntry?: ((entry: InspectorContentEntry) => void) | undefined;
  onReturn: () => void;
}
const NO_SPANS: readonly PlaybackWordSpan[] = [];
export function PlaybackFeedList({
  index,
  frame,
  liveFrame,
  browsing,
  onBrowse,
  onSeek,
  onNavigateEntry,
  onReturn,
}: PlaybackFeedListProps) {
  const [viewport, setViewport] = useState<HTMLDivElement | null>(null);
  const list = useRef<HTMLOListElement>(null);
  const virtual = index.entries.length > 40;
  const getScrollElement = useCallback(() => viewport, [viewport]);
  const getItemKey = useCallback(
    (i: number) => index.entries[i]?.id ?? i,
    [index],
  );
  const estimateSize = useCallback(
    (i: number) => {
      const entry = index.entries[i];
      if (entry?.kind === 'activity' || entry?.kind === 'focus') return 28;
      if (
        entry?.kind === 'speech' &&
        !playbackSpeechStartsGroup(entry, index.entries[i - 1])
      )
        return 60;
      return 76;
    },
    [index],
  );
  const positions = useMemo(
    () => new Map(index.entries.map((entry, i) => [entry.id, i])),
    [index],
  );
  const virtualizer = useVirtualizer({
    count: index.entries.length,
    getScrollElement,
    getItemKey,
    estimateSize,
    enabled: virtual,
    overscan: 6,
    useAnimationFrameWithResizeObserver: true,
    paddingStart: 12,
    paddingEnd: 12,
  });
  virtualizer.shouldAdjustScrollPositionOnItemSizeChange = browsing
    ? () => false
    : undefined;
  const browse = useCallback(() => {
    if (browsing) return;
    // Supersede pending scroll-to-index reconciliation when the user takes over.
    if (virtual && viewport) virtualizer.scrollToOffset(viewport.scrollTop);
    onBrowse();
  }, [browsing, virtual, viewport, virtualizer, onBrowse]);
  const keyDown = useCallback(
    (event: KeyboardEvent) => {
      if (
        event.defaultPrevented ||
        event.altKey ||
        event.ctrlKey ||
        event.metaKey
      )
        return;
      if (
        ['ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 'Home', 'End'].includes(
          event.key,
        ) ||
        (event.key === ' ' && event.target === list.current)
      ) {
        event.stopPropagation();
        browse();
        if (
          (event.key === 'Home' || event.key === 'End') &&
          event.target === list.current &&
          viewport
        ) {
          event.preventDefault();
          if (virtual)
            virtualizer.scrollToIndex(
              event.key === 'Home' ? 0 : index.entries.length - 1,
              { align: event.key === 'Home' ? 'start' : 'end' },
            );
          else
            viewport.scrollTo({
              top: event.key === 'Home' ? 0 : viewport.scrollHeight,
              behavior: 'instant',
            });
        }
      }
    },
    [browse, viewport, virtual, virtualizer, index.entries.length],
  );
  const wheel = useCallback(
    (event: WheelEvent) => {
      if (event.deltaY !== 0 && !event.ctrlKey && !event.defaultPrevented)
        browse();
    },
    [browse],
  );
  const pointerDown = useCallback(
    (event: PointerEvent) => {
      if (
        event.target instanceof Element &&
        event.target.closest('[data-slot="scroll-area-scrollbar"]')
      )
        browse();
    },
    [browse],
  );
  useEffect(() => {
    if (browsing || !viewport) return;
    const position =
      frame.currentId === null ? 0 : positions.get(frame.currentId);
    if (position === undefined) return;
    if (virtual) {
      virtualizer.scrollToIndex(position, {
        align: 'center',
        behavior: 'auto',
      });
      return;
    }
    const item = list.current?.children[position];
    if (item instanceof HTMLElement)
      viewport.scrollTo({
        top: Math.max(
          0,
          viewport.scrollTop +
            item.getBoundingClientRect().top -
            viewport.getBoundingClientRect().top -
            viewport.clientHeight / 3,
        ),
        behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches
          ? 'instant'
          : 'smooth',
      });
  }, [frame.currentId, browsing, viewport, positions, virtual, virtualizer]);
  const items = virtual
    ? virtualizer.getVirtualItems()
    : index.entries.map((_, i) => ({ index: i, key: getItemKey(i), start: 0 }));
  return (
    <div className="relative flex min-h-0 min-w-0 flex-1 flex-col">
      <ScrollArea
        type="always"
        viewportRef={setViewport}
        onWheel={wheel}
        onTouchMove={browse}
        onKeyDown={keyDown}
        onPointerDown={pointerDown}
        className="[&_[data-slot=scroll-area-thumb]]:bg-muted-foreground/60 min-h-0 min-w-0 flex-1 [&_[data-slot=scroll-area-viewport]>div]:block!"
      >
        <ol
          ref={list}
          tabIndex={0}
          aria-label="Teleprompter entries"
          className="focus-visible:ring-ring relative min-w-0 px-6 py-3 outline-none focus-visible:ring-1"
          style={virtual ? { height: virtualizer.getTotalSize() } : undefined}
        >
          {items.map((item) => {
            const entry = index.entries[item.index];
            if (!entry) return null;
            return (
              <PlaybackFeedRow
                key={item.key}
                entry={entry}
                position={item.index}
                count={index.entries.length}
                virtual={virtual}
                start={item.start}
                measure={virtual ? virtualizer.measureElement : undefined}
                showSourceHeading={playbackSpeechStartsGroup(
                  entry,
                  index.entries[item.index - 1],
                )}
                spans={index.spans.get(entry.id) ?? NO_SPANS}
                activeWordId={frame.activeWords.get(entry.id)}
                phase={
                  entry.startMs > frame.time
                    ? 'upcoming'
                    : frame.activeEntries.has(entry.id)
                      ? 'current'
                      : 'past'
                }
                onSeek={onSeek}
                onNavigateEntry={onNavigateEntry}
              />
            );
          })}
        </ol>
      </ScrollArea>
      <PlaybackRuler
        viewport={viewport}
        list={list}
        positions={positions}
        frame={liveFrame}
        firstStartMs={index.entries[0]?.startMs}
        lastEndMs={index.entries.at(-1)?.endMs}
        onReturn={onReturn}
      />
    </div>
  );
}

const PlaybackFeedRow = memo(function PlaybackFeedRow({
  entry,
  position,
  count,
  virtual,
  start,
  measure,
  showSourceHeading,
  spans,
  activeWordId,
  phase,
  onSeek,
  onNavigateEntry,
}: {
  entry: InspectorContentEntry;
  position: number;
  count: number;
  virtual: boolean;
  start: number;
  measure: ((element: HTMLElement | null) => void) | undefined;
  showSourceHeading: boolean;
  spans: readonly PlaybackWordSpan[];
  activeWordId: string | undefined;
  phase: 'past' | 'current' | 'upcoming';
  onSeek: (time: number) => void;
  onNavigateEntry: ((entry: InspectorContentEntry) => void) | undefined;
}) {
  return (
    <li
      data-index={position}
      ref={measure}
      aria-posinset={position + 1}
      aria-setsize={count}
      data-virtual={virtual}
      className="grid min-w-0 grid-cols-[4rem_minmax(0,1fr)] gap-2 data-[virtual=true]:absolute data-[virtual=true]:top-0 data-[virtual=true]:right-6 data-[virtual=true]:left-6"
      style={virtual ? { transform: `translateY(${start}px)` } : undefined}
    >
      <PlaybackTimeTick
        entry={entry}
        onSeek={onSeek}
        onNavigateEntry={onNavigateEntry}
      />
      <PlaybackFeedEntry
        entry={entry}
        showSourceHeading={showSourceHeading}
        spans={spans}
        activeWordId={activeWordId}
        phase={phase}
        onSeek={onSeek}
        onNavigateEntry={onNavigateEntry}
      />
    </li>
  );
});
