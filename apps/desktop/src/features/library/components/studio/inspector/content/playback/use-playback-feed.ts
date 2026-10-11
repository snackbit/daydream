import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { InspectorContentIndex } from '../inspector-content';
import {
  buildPlaybackFeedIndex,
  playbackFeedFrame,
  type PlaybackFeedFrame,
  type PlaybackFeedIndex,
} from './playback-feed-model';

export interface PlaybackFeedInput {
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
}: PlaybackFeedInput) {
  const prepared = useMemo(() => buildPlaybackFeedIndex(index), [index]);
  const anchored = useMemo(
    () => playbackFeedFrame(prepared, time),
    [prepared, time],
  );
  const [sample, setSample] = useState<OwnedFrame | null>(null);
  const [browsingOwner, setBrowsingOwner] = useState<FeedOwner | null>(null);
  const live =
    readPlaybackTimeMs &&
    sample?.index === prepared &&
    sample.seekRevision === seekRevision &&
    (isPlaying || sample.anchorTime === time)
      ? sample.frame
      : anchored;
  const browsing =
    browsingOwner?.index === prepared &&
    browsingOwner.seekRevision === seekRevision;
  const lastSample = useRef(-Infinity);
  useEffect(() => {
    if (!isPlaying || !readPlaybackTimeMs) return;
    let handle = requestAnimationFrame(function tick(now) {
      if (now - lastSample.current >= 1000 / 30) {
        lastSample.current = now;
        const next = playbackFeedFrame(prepared, readPlaybackTimeMs());
        setSample((current) =>
          current?.index === prepared &&
          current.seekRevision === seekRevision &&
          current.anchorTime === time &&
          current.frame.signature === next.signature
            ? current
            : { index: prepared, seekRevision, anchorTime: time, frame: next },
        );
      }
      handle = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(handle);
  }, [prepared, time, isPlaying, readPlaybackTimeMs, seekRevision]);
  const browse = useCallback(
    () =>
      setBrowsingOwner((current) =>
        current?.index === prepared && current.seekRevision === seekRevision
          ? current
          : { index: prepared, seekRevision },
      ),
    [prepared, seekRevision],
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
