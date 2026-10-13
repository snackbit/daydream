import { useCallback, useEffect, useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  useStudioResultRange,
  useStudioResultWindowStore,
} from '../../studio-result-window-store';
import type {
  InspectorContentIndex,
  InspectorContentEntry,
} from './inspector-content';
import { PlaybackFeedList } from './playback/playback-feed-list';
import { playbackTimestamp } from './playback/playback-feed-model';
import { ReviewFilters, type ReviewCategory } from './playback/review-filters';
import { usePlaybackFeed } from './playback/use-playback-feed';

export interface PlaybackInspectorFeedProps {
  active?: boolean;
  durationMs?: number | undefined;
  ownerId?: string | undefined;
  index: InspectorContentIndex;
  time: number;
  isPlaying: boolean;
  onSeek: (time: number) => void;
  onNavigateEntry?: ((entry: InspectorContentEntry) => void) | undefined;
  readPlaybackTimeMs?: (() => number) | undefined;
  seekRevision?: number | undefined;
}
export function PlaybackInspectorFeed({
  index,
  time,
  isPlaying,
  onSeek,
  onNavigateEntry,
  readPlaybackTimeMs,
  seekRevision = 0,
  ownerId,
  durationMs,
  active = true,
}: PlaybackInspectorFeedProps) {
  const readWindow = useStudioResultRange(ownerId ?? 'standalone');
  const publishReview = useStudioResultWindowStore((store) => store.setReview);
  const [categories, setCategories] = useState<ReadonlySet<ReviewCategory>>(
    () => new Set(['speech', 'observation', 'activity']),
  );
  const changeCategory = useCallback(
    (category: ReviewCategory, enabled: boolean) => {
      setCategories((current) => {
        const next = new Set(current);
        if (enabled) next.add(category);
        else next.delete(category);
        return next;
      });
    },
    [],
  );
  const filtered = useMemo(() => {
    const entries = index.entries.filter(
      (entry) =>
        (!ownerId ||
          (entry.startMs < (readWindow.reviewEndOffsetMs ?? Infinity) &&
            entry.endMs > (readWindow.reviewStartOffsetMs ?? 0))) &&
        categories.has(
          entry.kind === 'speech'
            ? 'speech'
            : entry.kind === 'observation'
              ? 'observation'
              : 'activity',
        ),
    );
    const speechBySource = new Map<string, InspectorContentEntry[]>();
    for (const entry of entries)
      if (entry.kind === 'speech') {
        const source = speechBySource.get(entry.trackId) ?? [];
        source.push(entry);
        speechBySource.set(entry.trackId, source);
      }
    return {
      entries,
      speechBySource,
      focus: entries.filter((entry) => entry.kind === 'focus'),
    };
  }, [
    index,
    categories,
    ownerId,
    readWindow.reviewStartOffsetMs,
    readWindow.reviewEndOffsetMs,
  ]);
  // Timeline page reads can republish identical Review content. Keep its row
  // identities and measurements while validating every visible fact, including
  // changed privacy eligibility, through the filtered content signature.
  const signature = useMemo(
    () => JSON.stringify([ownerId, filtered.entries]),
    [ownerId, filtered],
  );
  const [reviewSnapshot, setReviewSnapshot] = useState({
    signature,
    index: filtered,
  });
  if (reviewSnapshot.signature !== signature)
    setReviewSnapshot({ signature, index: filtered });
  const reviewIndex =
    reviewSnapshot.signature === signature ? reviewSnapshot.index : filtered;
  const feed = usePlaybackFeed({
    index: reviewIndex,
    time,
    isPlaying,
    readPlaybackTimeMs,
    seekRevision,
    ownerId,
  });
  useEffect(() => {
    if (active && ownerId && !feed.browsing)
      publishReview(
        ownerId,
        readPlaybackTimeMs ? readPlaybackTimeMs() : feed.liveFrame.time,
      );
  }, [
    active,
    ownerId,
    feed.browsing,
    feed.liveFrame.time,
    publishReview,
    readPlaybackTimeMs,
  ]);
  const browse = feed.browse;
  const earlier = useCallback(() => {
    if (!ownerId) return;
    browse();
    publishReview(
      ownerId,
      Math.max(0, (readWindow.reviewStartOffsetMs ?? 0) - 90_000),
    );
  }, [ownerId, browse, publishReview, readWindow.reviewStartOffsetMs]);
  const later = useCallback(() => {
    if (!ownerId) return;
    browse();
    publishReview(ownerId, (readWindow.reviewEndOffsetMs ?? 0) + 90_000);
  }, [ownerId, browse, publishReview, readWindow.reviewEndOffsetMs]);
  return (
    <section
      aria-label="Teleprompter"
      className="flex h-full min-h-0 min-w-0 flex-1 flex-col"
    >
      <div className="flex shrink-0 items-center justify-between px-4">
        <Button
          size="sm"
          variant="ghost"
          aria-label={feed.browsing ? 'Resume auto-scroll' : 'Follow playback'}
          aria-pressed={!feed.browsing}
          onClick={feed.resume}
        >
          Follow playback
        </Button>
        <ReviewFilters enabled={categories} onChange={changeCategory} />
      </div>
      {feed.index.entries.length > 0 || ownerId ? (
        <div className="flex min-h-7 shrink-0 items-center justify-between gap-2 px-6 pt-1">
          <span
            role="timer"
            aria-label="Playback position"
            className="text-annotation-gold w-16 shrink-0 pr-3 text-right text-[11px] tabular-nums"
          >
            {playbackTimestamp(feed.liveFrame.time)}
          </span>
          {ownerId ? (
            <div className="flex gap-1">
              <Button
                size="sm"
                variant="ghost"
                onClick={earlier}
                disabled={(readWindow.reviewStartOffsetMs ?? 0) === 0}
              >
                Earlier
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={later}
                disabled={
                  durationMs !== undefined &&
                  (readWindow.reviewEndOffsetMs ?? 0) >= durationMs
                }
              >
                Later
              </Button>
            </div>
          ) : null}
        </div>
      ) : null}
      {feed.index.entries.length === 0 ? (
        <p className="text-muted-foreground px-6 py-3 text-sm">
          No transcripts or AI observations yet.
        </p>
      ) : (
        <PlaybackFeedList
          index={feed.index}
          frame={feed.frame}
          liveFrame={feed.liveFrame}
          browsing={feed.browsing}
          onBrowse={feed.browse}
          onSeek={onSeek}
          onNavigateEntry={onNavigateEntry}
          onReturn={feed.resume}
        />
      )}
    </section>
  );
}
