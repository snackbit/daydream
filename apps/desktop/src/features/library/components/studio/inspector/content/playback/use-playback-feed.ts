import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { InspectorContentIndex } from '../inspector-content';
import {
  buildPlaybackFeedIndex,
  playbackFeedFrame,
  type PlaybackFeedFrame,
  type PlaybackFeedIndex,
} from './playback-feed-model';

export interface PlaybackFeedInput {
  ownerId?: string | undefined;
  index: InspectorContentIndex;
  time: number;
  isPlaying: boolean;
  seekRevision: number;
  readPlaybackTimeMs: (() => number) | undefined;
}
interface FeedOwner {
  index: PlaybackFeedIndex;
  seekRevision: number;
}
interface OwnedFrame extends FeedOwner {
  anchorTime: number;
  frame: PlaybackFeedFrame;
}

export function usePlaybackFeed({
  index,
  time,
  isPlaying,
  seekRevision,
  readPlaybackTimeMs,
  ownerId,
}: PlaybackFeedInput) {
  const ownerKey = ownerId ?? 'recording';
  const prepared = useMemo(() => buildPlaybackFeedIndex(index), [index]);
  const anchored = useMemo(
    () => playbackFeedFrame(prepared, time),
    [prepared, time],
  );
  const [sample, setSample] = useState<OwnedFrame | null>(null);
  const [browsingOwner, setBrowsingOwner] = useState<{
    ownerId: string;
    seekRevision: number;
  } | null>(null);
  const live =
    readPlaybackTimeMs &&
    sample?.index === prepared &&
    sample.seekRevision === seekRevision &&
    (isPlaying || sample.anchorTime === time)
      ? sample.frame
      : anchored;
  const browsing =
    (prepared.entries.length > 0 || ownerId !== undefined) &&
    browsingOwner?.ownerId === ownerKey &&
    browsingOwner.seekRevision === seekRevision;
  const lastSample = useRef(-Infinity);
  useEffect(() => {
    if (!isPlaying || !readPlaybackTimeMs) return;
    let signature: string | undefined;
    let handle = requestAnimationFrame(function tick(now) {
      if (now - lastSample.current >= 1000 / 30) {
        lastSample.current = now;
        const next = playbackFeedFrame(prepared, readPlaybackTimeMs());
        if (signature !== next.signature) {
          signature = next.signature;
          setSample({
            index: prepared,
            seekRevision,
            anchorTime: time,
            frame: next,
          });
        }
      }
      handle = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(handle);
  }, [prepared, time, isPlaying, readPlaybackTimeMs, seekRevision]);
  const browse = useCallback(
    () =>
      setBrowsingOwner((current) =>
        current?.ownerId === ownerKey && current.seekRevision === seekRevision
          ? current
          : { ownerId: ownerKey, seekRevision },
      ),
    [ownerKey, seekRevision],
  );
  const resume = useCallback(() => {
    setSample(
      isPlaying && readPlaybackTimeMs
        ? {
            index: prepared,
            seekRevision,
            anchorTime: time,
            frame: playbackFeedFrame(prepared, readPlaybackTimeMs()),
          }
        : null,
    );
    setBrowsingOwner(null);
  }, [isPlaying, readPlaybackTimeMs, prepared, seekRevision, time]);
  return {
    index: prepared,
    frame: live,
    liveFrame: live,
    browsing,
    browse,
    resume,
  };
}
