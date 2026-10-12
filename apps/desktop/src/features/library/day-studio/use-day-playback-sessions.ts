import { useCallback, useEffect, useRef, useState } from 'react';
import type { LibraryPlaybackAssets } from '@daydream/shared';
import {
  closeLibraryPlaybackSession,
  createLibraryPlaybackSession,
} from '../library-client';
import type { DayTimelineModel } from './day-timeline';

/** Session metadata is cheap; only the current and next media groups decode. */
export function useDayPlaybackSessions(model: DayTimelineModel) {
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
  const ensure = useCallback(async (recordingId: string) => {
    const current = owner.current;
    if (!current || !current.active || !current.allowed.has(recordingId))
      throw new Error('Day playback has closed');
    const saved = current.values.get(recordingId);
    if (saved) return saved;
    const revision = current.audioRevisions.get(recordingId);
    let request = current.pending.get(recordingId);
    if (!request) {
      request = createLibraryPlaybackSession({ recordingId });
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
      if (!current.values.has(recordingId)) {
        current.values.set(recordingId, value);
        setSessions(new Map(current.values));
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
  return { sessions, error, ensure };
}
