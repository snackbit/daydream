import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  AudioCleanupConfig,
  AudioCleanupPreview,
  AudioCleanupState,
  LibraryRecordingDetail,
} from '@daydream/shared';
import { closeLibraryPlaybackSession } from '../../../library-client';
import {
  applyAudioCleanup,
  cancelAudioCleanup,
  getAudioCleanup,
  previewAudioCleanup,
} from './audio-cleanup-client';

export const DEFAULT_CLEANUP: AudioCleanupConfig = {
  mode: 'off',
  matchThreshold: 0.65,
  amount: 1,
};
export interface AudioCleanupOptions {
  detail: LibraryRecordingDetail;
  readRange: () => { startOffsetMs: number; endOffsetMs: number };
  pausePlayback: () => void;
  refreshPlayback: () => void;
  restorePlayback?: () => void;
}
export function useAudioCleanup({
  detail,
  readRange,
  pausePlayback,
  refreshPlayback,
  restorePlayback,
}: AudioCleanupOptions) {
  const [owner, setOwner] = useState(detail.id);
  const [opened, setOpened] = useState(false);
  const [saved, setSaved] = useState<AudioCleanupState | null>(
    detail.audioCleanup ?? null,
  );
  const [draft, setDraft] = useState<AudioCleanupConfig>(
    detail.audioCleanup?.config ?? DEFAULT_CLEANUP,
  );
  const [preview, setPreview] = useState<AudioCleanupPreview | null>(null);
  const [busy, setBusy] = useState<
    'load' | 'preview' | 'apply' | 'cancel' | null
  >(null);
  const [error, setError] = useState<string | null>(null);
  const generation = useRef(0);
  const previewSession = useRef<string | null>(null);
  const disposePreview = useCallback(() => {
    const session = previewSession.current;
    previewSession.current = null;
    if (session)
      void closeLibraryPlaybackSession({ sessionId: session }).catch(() => {
        setError(
          'The comparison could not close. Refresh playback before trying again.',
        );
      });
    setPreview(null);
  }, []);
  useEffect(() => {
    return () => {
      generation.current++;
      const session = previewSession.current;
      previewSession.current = null;
      if (session)
        void closeLibraryPlaybackSession({ sessionId: session }).catch(
          () => {},
        );
    };
  }, [detail.id]);
  const state = owner === detail.id ? saved : (detail.audioCleanup ?? null);
  useEffect(() => {
    if (detail.audioCleanup) {
      setOwner(detail.id);
      setSaved(detail.audioCleanup);
    }
  }, [detail.id, detail.audioCleanup]);
  useEffect(() => {
    if (state?.phase !== 'running') return;
    let live = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = async () => {
      try {
        const next = await getAudioCleanup(detail.id);
        if (!live) return;
        setOwner(detail.id);
        setSaved(next);
        if (next.phase === 'ready' && next.revision !== state.revision)
          refreshPlayback();
        if (next.phase === 'running')
          timer = setTimeout(() => void poll(), 750);
      } catch {
        if (live)
          setError(
            'Cleanup progress could not be read. Reopen this dialog to reconnect.',
          );
      }
    };
    timer = setTimeout(() => void poll(), 750);
    return () => {
      live = false;
      if (timer) clearTimeout(timer);
    };
  }, [detail.id, state?.phase, state?.revision, refreshPlayback]);
  const open = useCallback(async () => {
    pausePlayback();
    const request = ++generation.current;
    setOwner(detail.id);
    setOpened(true);
    setBusy('load');
    setError(null);
    disposePreview();
    try {
      const next = await getAudioCleanup(detail.id);
      if (request !== generation.current) return;
      setSaved(next);
      setDraft(next.pendingConfig ?? next.config);
    } catch {
      if (request === generation.current)
        setError(
          'Cleanup settings could not be loaded. Close and reopen to retry.',
        );
    } finally {
      if (request === generation.current) setBusy(null);
    }
  }, [detail.id, pausePlayback, disposePreview]);
  const close = useCallback(() => {
    if (busy === 'apply' || busy === 'cancel') return;
    generation.current++;
    disposePreview();
    setOpened(false);
    setBusy(null);
    restorePlayback?.();
  }, [busy, disposePreview, restorePlayback]);
  const change = useCallback(
    (next: AudioCleanupConfig) => {
      disposePreview();
      setDraft(next);
      setError(null);
    },
    [disposePreview],
  );
  const compare = useCallback(async () => {
    const request = ++generation.current;
    disposePreview();
    setBusy('preview');
    setError(null);
    try {
      const next = await previewAudioCleanup({
        recordingId: detail.id,
        config: draft,
        ...readRange(),
      });
      if (request !== generation.current) {
        await closeLibraryPlaybackSession({ sessionId: next.sessionId });
        return;
      }
      previewSession.current = next.sessionId;
      setPreview(next);
    } catch {
      if (request === generation.current)
        setError(
          'This range could not be previewed. Choose available microphone audio and try again.',
        );
    } finally {
      if (request === generation.current) setBusy(null);
    }
  }, [detail.id, draft, readRange, disposePreview]);
  const apply = useCallback(async () => {
    const request = ++generation.current;
    disposePreview();
    setBusy('apply');
    setError(null);
    try {
      const next = await applyAudioCleanup({
        recordingId: detail.id,
        config: draft,
      });
      if (request !== generation.current) return;
      setSaved(next);
      if (draft.mode === 'off') refreshPlayback();
    } catch {
      if (request === generation.current)
        setError(
          'Cleanup could not start. Finish recording, check the audio feeds, and retry.',
        );
    } finally {
      if (request === generation.current) setBusy(null);
    }
  }, [detail.id, draft, refreshPlayback, disposePreview]);
  const cancel = useCallback(async () => {
    const request = ++generation.current;
    setBusy('cancel');
    setError(null);
    try {
      const next = await cancelAudioCleanup(detail.id);
      if (request === generation.current) setSaved(next);
    } catch {
      if (request === generation.current)
        setError(
          'Cleanup is still stopping. Retry cancellation before starting another cleanup.',
        );
    } finally {
      if (request === generation.current) setBusy(null);
    }
  }, [detail.id]);
  return {
    opened: owner === detail.id && opened,
    state,
    draft,
    preview,
    busy,
    error,
    open,
    close,
    change,
    compare,
    apply,
    cancel,
  };
}
export type AudioCleanupController = ReturnType<typeof useAudioCleanup>;
