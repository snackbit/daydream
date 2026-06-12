import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  CreateLibraryRedactionRangeInput,
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
} from '@daydream/shared';
import { startRecordingAiProcessing } from '@/features/ai/ai-client';
import { useRecordingAiProcessingState } from '@/features/ai/use-recording-ai-processing-state';
import {
  closeLibraryPlaybackSession,
  createLibraryPlaybackSession,
  createLibraryRedactions,
  deleteLibraryRecording,
  deleteLibraryRedaction,
  getLibraryRecording,
  renameLibraryRecording,
  unredactLibraryRedactions,
} from '../library-client';
import { errorMessage } from './recording-studio-route-utils';

export function useRecordingStudioController({
  onDeleted,
  recordingId,
}: {
  onDeleted: () => Promise<void>;
  recordingId: string;
}) {
  const [detail, setDetail] = useState<LibraryRecordingDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [playbackAssets, setPlaybackAssets] =
    useState<LibraryPlaybackAssets | null>(null);
  const [status, setStatus] = useState<'error' | 'idle' | 'loading' | 'ready'>(
    'idle',
  );
  const [thumbnailStatus, setThumbnailStatus] = useState<
    'error' | 'idle' | 'loading' | 'ready'
  >('idle');
  const [processingStartBusy, setProcessingStartBusy] = useState(false);
  const aiProcessing = useRecordingAiProcessingState(recordingId);
  const loadRunRef = useRef(0);
  const playbackSessionIdRef = useRef<string | null>(null);

  const load = useCallback(async () => {
    const loadRunId = loadRunRef.current + 1;
    loadRunRef.current = loadRunId;
    const isCurrentLoad = () => loadRunRef.current === loadRunId;
    const previousSessionId = playbackSessionIdRef.current;
    playbackSessionIdRef.current = null;
    if (previousSessionId) {
      void closeLibraryPlaybackSession({ sessionId: previousSessionId });
    }

    setError(null);
    setStatus('loading');
    setThumbnailStatus('idle');
    setPlaybackAssets(null);
    try {
      const nextDetail = await getLibraryRecording(recordingId);
      if (!isCurrentLoad()) {
        return;
      }
      setDetail(nextDetail);
      setStatus('ready');
      if (nextDetail.availability === 'available') {
        createLibraryPlaybackSession({ recordingId })
          .then((assets) => {
            if (!isCurrentLoad() || assets.recordingId !== recordingId) {
              if (assets.sessionId) {
                void closeLibraryPlaybackSession({
                  sessionId: assets.sessionId,
                });
              }
              return;
            }
            playbackSessionIdRef.current = assets.sessionId;
            setPlaybackAssets(assets);
            setThumbnailStatus(assets.error === null ? 'ready' : 'error');
          })
          .catch((playbackError: unknown) => {
            if (!isCurrentLoad()) {
              return;
            }
            setPlaybackAssets({
              error: errorMessage(playbackError),
              recordingId,
              segments: [],
              sessionId: '',
            });
          });
      }
    } catch (loadError) {
      if (!isCurrentLoad()) {
        return;
      }
      setError(errorMessage(loadError));
      setStatus('error');
    }
  }, [recordingId]);

  const confirmDelete = useCallback(async () => {
    setError(null);
    try {
      await deleteLibraryRecording({ recordingId });
      await onDeleted();
    } catch (deleteError) {
      setError(errorMessage(deleteError));
    }
  }, [onDeleted, recordingId]);

  const confirmRename = useCallback(
    async (nextTitle: string) => {
      if (!detail) {
        return;
      }
      const trimmedTitle = nextTitle.trim();
      setError(null);
      try {
        const nextDetail = await renameLibraryRecording({
          recordingId: detail.id,
          title: trimmedTitle.length > 0 ? trimmedTitle : null,
        });
        setDetail(nextDetail);
      } catch (renameError) {
        setError(errorMessage(renameError));
        throw renameError;
      }
    },
    [detail],
  );

  const createRedactions = useCallback(
    async (ranges: CreateLibraryRedactionRangeInput[]) => {
      if (!detail) {
        return;
      }
      setError(null);
      try {
        setDetail(
          await createLibraryRedactions({
            ranges,
            recordingId: detail.id,
          }),
        );
      } catch (redactionError) {
        const message = errorMessage(redactionError);
        setError(message);
        throw new Error(message);
      }
    },
    [detail],
  );

  const removeRedaction = useCallback(
    async (redactionId: string) => {
      if (!detail) {
        return;
      }
      setError(null);
      try {
        setDetail(
          await deleteLibraryRedaction({
            recordingId: detail.id,
            redactionId,
          }),
        );
      } catch (redactionError) {
        const message = errorMessage(redactionError);
        setError(message);
        throw new Error(message);
      }
    },
    [detail],
  );

  const unredactRanges = useCallback(
    async (ranges: CreateLibraryRedactionRangeInput[]) => {
      if (!detail) {
        return;
      }
      setError(null);
      try {
        setDetail(
          await unredactLibraryRedactions({
            ranges,
            recordingId: detail.id,
          }),
        );
      } catch (redactionError) {
        const message = errorMessage(redactionError);
        setError(message);
        throw new Error(message);
      }
    },
    [detail],
  );

  const startAiProcessing = useCallback(async () => {
    if (!detail) {
      return;
    }
    setError(null);
    setProcessingStartBusy(true);
    try {
      await startRecordingAiProcessing({ recordingId: detail.id });
      await aiProcessing.reload();
    } catch (processingError) {
      setError(errorMessage(processingError));
    } finally {
      setProcessingStartBusy(false);
    }
  }, [aiProcessing, detail]);

  useEffect(() => {
    void Promise.resolve().then(load);
  }, [load]);

  useEffect(
    () => () => {
      loadRunRef.current += 1;
      const sessionId = playbackSessionIdRef.current;
      playbackSessionIdRef.current = null;
      if (sessionId) {
        void closeLibraryPlaybackSession({ sessionId });
      }
    },
    [],
  );

  return {
    actions: {
      confirmDelete,
      confirmRename,
      createRedactions,
      load,
      removeRedaction,
      startAiProcessing,
      unredactRanges,
    },
    state: {
      aiProcessing,
      detail,
      error,
      playbackAssets,
      processingStartBusy,
      status,
      thumbnailStatus,
    },
  };
}
