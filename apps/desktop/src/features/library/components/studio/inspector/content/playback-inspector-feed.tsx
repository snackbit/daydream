import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Button } from '@/components/ui/button';
import { InspectorSection } from '../inspector-section';
import {
  inspectorPlaybackWindow,
  type InspectorContentIndex,
} from './inspector-content';
import { InspectorContentEntryCard } from './inspector-content-entry';

export interface PlaybackInspectorFeedProps {
  index: InspectorContentIndex;
  time: number;
  isPlaying: boolean;
  onSeek: (time: number) => void;
}
export function PlaybackInspectorFeed({
  index,
  time,
  isPlaying,
  onSeek,
}: PlaybackInspectorFeedProps) {
  const live = useMemo(
    () => inspectorPlaybackWindow(index, time),
    [index, time],
  );
  const [frozen, setFrozen] = useState<{
    index: InspectorContentIndex;
    content: typeof live;
    time: number;
  } | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const previousTime = useRef(time),
    previousPlaying = useRef(isPlaying);
  useEffect(() => {
    if (
      Math.abs(time - previousTime.current) > 1500 ||
      (!previousPlaying.current && isPlaying)
    )
      setFrozen(null);
    previousTime.current = time;
    previousPlaying.current = isPlaying;
  }, [time, isPlaying]);
  const browsing = frozen?.index === index ? frozen : null;
  const content = browsing?.content ?? live;
  const displayTime = browsing?.time ?? time;
  const resume = useCallback(() => setFrozen(null), []);
  const browse = useCallback(
    () =>
      setFrozen((current) =>
        current?.index === index ? current : { index, content: live, time },
      ),
    [index, live, time],
  );
  useEffect(() => {
    if (browsing || !isPlaying || !content.currentId) return;
    const item = Array.from(
      root.current?.querySelectorAll<HTMLElement>('[data-inspector-entry]') ??
        [],
    ).find((element) => element.dataset.inspectorEntry === content.currentId);
    const viewport = root.current?.closest(
      '[data-slot="scroll-area-viewport"]',
    );
    if (item instanceof HTMLElement && viewport instanceof HTMLElement) {
      const top =
        viewport.scrollTop +
        item.getBoundingClientRect().top -
        viewport.getBoundingClientRect().top -
        viewport.clientHeight / 3;
      viewport.scrollTo({
        top: Math.max(0, top),
        behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches
          ? 'instant'
          : 'smooth',
      });
    }
  }, [content.currentId, browsing, isPlaying]);
  return (
    <InspectorSection title="Playback feed">
      <div ref={root} onWheel={browse} onTouchMove={browse}>
        <div className="mb-3 flex items-center justify-between gap-2">
          <span className="text-muted-foreground text-xs">
            {browsing ? 'Browsing context' : 'Following playback'}
          </span>
          {browsing ? (
            <Button size="sm" variant="outline" onClick={resume}>
              Return to playback
            </Button>
          ) : null}
        </div>
        {content.focus ? (
          <p className="text-muted-foreground mb-4 rounded-lg border px-3 py-2 text-xs">
            Current window · {content.focus.text}
          </p>
        ) : null}
        {content.entries.length === 0 ? (
          <p className="text-muted-foreground text-sm">
            No text or activity is available yet. Process this recording to add
            transcripts and screen text.
          </p>
        ) : (
          <ol className="space-y-1">
            {content.entries.map((entry, i) => {
              const phase =
                entry.startMs > displayTime
                  ? 'upcoming'
                  : entry.endMs > displayTime
                    ? 'current'
                    : 'past';
              const showNow =
                entry.startMs > displayTime &&
                (i === 0 ||
                  (content.entries[i - 1]?.startMs ?? 0) <= displayTime);
              return (
                <FeedEntry
                  key={entry.id}
                  entry={entry}
                  phase={phase}
                  current={entry.id === content.currentId}
                  onSeek={onSeek}
                  showNow={showNow}
                />
              );
            })}
          </ol>
        )}
      </div>
    </InspectorSection>
  );
}
function FeedEntry({
  showNow,
  ...props
}: React.ComponentProps<typeof InspectorContentEntryCard> & {
  showNow: boolean;
}) {
  return (
    <>
      {showNow ? (
        <li className="text-muted-foreground flex items-center gap-2 py-2 text-xs">
          <span className="bg-border h-px flex-1" />
          Now
          <span className="bg-border h-px flex-1" />
        </li>
      ) : null}
      <InspectorContentEntryCard {...props} />
    </>
  );
}
