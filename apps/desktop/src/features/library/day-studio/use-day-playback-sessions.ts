import { useCallback, useEffect, useRef, useState } from 'react';
import type { LibraryPlaybackAssets } from '@daydream/shared';
import { useStudioResultRange } from '../components/studio/results/studio-result-window-store';
import {
  closeLibraryPlaybackSession,
  createLibraryPlaybackSession,
  listenLibraryMediaPrepared,
} from '../library-client';
import type { DayTimelineModel } from './day-timeline';

/** Session metadata is cheap; only the current and next media groups decode. */
export function useDayPlaybackSessions(model: DayTimelineModel) {
  const viewport = useStudioResultRange(model.view.id);
  const [preparedIds, setPreparedIds] = useState<ReadonlySet<string>>(
    new Set(),
  );
  const [sessions, setSessions] = useState(
      new Map<string, LibraryPlaybackAssets>(),
    ),
    [error, setError] = useState<string | null>(null);
  const owner = useRef<{
    active: boolean;
    allowed: Set<string>;
    values: Map<string, LibraryPlaybackAssets>;
    pending: Map<string, Promise<LibraryPlaybackAssets>>;
    retired: Set<string>;
    audioRevisions: Map<string, string>;
  } | null>(null);
  const ownershipKey = `${model.manifest.dayId}:${model.manifest.privacyRevision}`;
  useEffect(() => {
    const current = {
      active: true,
      allowed: new Set<string>(),
      values: new Map<string, LibraryPlaybackAssets>(),
      pending: new Map<string, Promise<LibraryPlaybackAssets>>(),
      retired: new Set<string>(),
      audioRevisions: new Map<string, string>(),
    };
    owner.current = current;
    setSessions(new Map());
    setPreparedIds(new Set());
    setError(null);
    return () => {
      current.active = false;
      for (const value of current.values.values())
        if (value.sessionId) {
          current.retired.add(value.sessionId);
          void closeLibraryPlaybackSession({ sessionId: value.sessionId });
        }
    };
  }, [ownershipKey]);
  useEffect(() => {
    const current = owner.current;
    if (!current) return;
    current.allowed = new Set(
      model.manifest.recordings.map((recording) => recording.id),
    );
    const revisions = new Map(
      model.manifest.recordings.map((recording) => [
        recording.id,
        recording.audioCleanup?.revision ?? 'original',
      ]),
    );
    let removed = false;
    for (const [id, value] of current.values)
      if (
        !current.allowed.has(id) ||
        current.audioRevisions.get(id) !== revisions.get(id)
      ) {
        current.values.delete(id);
        removed = true;
        if (value.sessionId) {
          current.retired.add(value.sessionId);
          void closeLibraryPlaybackSession({ sessionId: value.sessionId });
        }
      }
    for (const [id, revision] of current.audioRevisions) {
      if (revision !== revisions.get(id)) current.pending.delete(id);
    }
    current.audioRevisions = revisions;
    if (removed) setSessions(new Map(current.values));
  }, [model.manifest.recordings]);
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void listenLibraryMediaPrepared((event) => {
      const current = owner.current;
      if (
        !active ||
        !current?.active ||
        !current.allowed.has(event.recordingId)
      )
        return;
      const value = current.values.get(event.recordingId);
      if (value?.segments.some((segment) => segment.canSeekInStream !== true))
        setPreparedIds((ids) => new Set(ids).add(event.recordingId));
    })
      .then((cleanup) => {
        if (active) unlisten = cleanup;
        else cleanup();
      })
      .catch(() => undefined);
    return () => {
      active = false;
      unlisten?.();
    };
  }, [ownershipKey]);
  const ensure = useCallback(async function ensureRecording(
    recordingId: string,
    refresh = false,
    accept?: () => boolean,
  ): Promise<LibraryPlaybackAssets> {
    const current = owner.current;
    if (!current || !current.active || !current.allowed.has(recordingId))
      throw new Error('Day playback has closed');
    const saved = current.values.get(recordingId);
    if (saved && !refresh) return saved;
    const preceding = current.pending.get(recordingId);
    if (refresh && preceding) {
      await preceding.catch(() => undefined);
      return ensureRecording(recordingId, true, accept);
    }
    const revision = current.audioRevisions.get(recordingId);
    let request = current.pending.get(recordingId);
    if (!request) {
      request = createLibraryPlaybackSession({
        recordingId,
        lazyThumbnails: true,
        includeRuns: false,
      });
      current.pending.set(recordingId, request);
    }
    try {
      const value = await request;
      if (
        !current.active ||
        !current.allowed.has(recordingId) ||
        current.audioRevisions.get(recordingId) !== revision
      ) {
        if (value.sessionId && !current.retired.has(value.sessionId)) {
          current.retired.add(value.sessionId);
          void closeLibraryPlaybackSession({ sessionId: value.sessionId });
        }
        throw new Error('Day playback has closed');
      }
      if (
        value.error ||
        value.segments.length === 0 ||
        value.recordingId !== recordingId
      ) {
        if (value.sessionId && !current.retired.has(value.sessionId)) {
          current.retired.add(value.sessionId);
          void closeLibraryPlaybackSession({ sessionId: value.sessionId });
        }
        throw new Error('Playback preparation failed');
      }
      if (saved && refresh && accept && !accept()) {
        if (value.sessionId && !current.retired.has(value.sessionId)) {
          current.retired.add(value.sessionId);
          void closeLibraryPlaybackSession({ sessionId: value.sessionId });
        }
        return saved;
      }
      if (!current.values.has(recordingId) || refresh) {
        current.values.set(recordingId, value);
        setSessions(new Map(current.values));
        setPreparedIds((ids) => {
          if (!ids.has(recordingId)) return ids;
          const next = new Set(ids);
          next.delete(recordingId);
          return next;
        });
        if (
          saved?.sessionId &&
          saved.sessionId !== value.sessionId &&
          !current.retired.has(saved.sessionId)
        ) {
          current.retired.add(saved.sessionId);
          void closeLibraryPlaybackSession({ sessionId: saved.sessionId });
        }
      }
      setError(null);
      return value;
    } catch (failure) {
      if (
        current.active &&
        current.allowed.has(recordingId) &&
        current.audioRevisions.get(recordingId) === revision
      )
        setError('Playback could not be prepared. Try refreshing the day.');
      throw failure;
    } finally {
      if (current.pending.get(recordingId) === request)
        current.pending.delete(recordingId);
    }
  }, []);
  useEffect(() => {
    if (!viewport.ready) return;
    let active = true;
    const ids = [
      ...new Set(
        model.spans
          .filter((span) => {
            const start = span.startMs - model.originMs;
            return (
              start < viewport.endOffsetMs &&
              start + span.durationMs > viewport.startOffsetMs
            );
          })
          .map((span) => span.recordingId),
      ),
    ];
    let index = 0;
    async function worker() {
      while (active) {
        const id = ids[index++];
        if (!id) return;
        try {
          await ensure(id);
        } catch {
          /* The session owner exposes the error. */
        }
      }
    }
    void Promise.all([worker(), worker()]);
    return () => {
      active = false;
    };
  }, [
    model,
    viewport.ready,
    viewport.startOffsetMs,
    viewport.endOffsetMs,
    ensure,
  ]);
  return { sessions, error, ensure, preparedIds };
}
