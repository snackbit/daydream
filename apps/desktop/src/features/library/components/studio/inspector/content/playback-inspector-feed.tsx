import { Button } from '@/components/ui/button';
import type {
  InspectorContentIndex,
  InspectorContentEntry,
} from './inspector-content';
import { PlaybackFeedList } from './playback/playback-feed-list';
import { playbackTimestamp } from './playback/playback-feed-model';
import { usePlaybackFeed } from './playback/use-playback-feed';

export interface PlaybackInspectorFeedProps {
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
}: PlaybackInspectorFeedProps) {
  const feed = usePlaybackFeed({
    index,
    time,
    isPlaying,
    readPlaybackTimeMs,
    seekRevision,
  });
  return (
    <section
      aria-label="Teleprompter"
      className="flex h-full min-h-0 min-w-0 flex-1 flex-col"
    >
      {feed.index.entries.length > 0 ? (
        <div className="flex min-h-7 shrink-0 items-center justify-between gap-2 px-6 pt-1">
          <span
            role="timer"
            aria-label="Playback position"
            className="text-annotation-gold w-16 shrink-0 pr-3 text-right text-[11px] tabular-nums"
          >
            {playbackTimestamp(feed.liveFrame.time)}
          </span>
          {feed.browsing ? (
            <Button
              size="sm"
              variant="ghost"
              className="h-auto px-1 py-0.5 text-xs"
              onClick={feed.resume}
            >
              Resume auto-scroll
            </Button>
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
