import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  CreateLibraryRedactionRangeInput,
  LibraryPlaybackAssets,
  LibraryRecordingDetail,
} from '@daydream/shared';
import {
  startRecordingAiProcessing,
  retryProcessingJob,
  resumeProcessingJob,
} from '@/features/ai/ai-client';
import { visibleOcrTimelineFrames } from '@/features/ai/ocr/ocr-timeline-visibility';
import { useOcrTimelineFrames } from '@/features/ai/ocr/use-ocr-timeline-frames';
import {
  visibleActivity,
  visibleAiProcessingState,
  visibleVisualObservations,
} from '@/features/ai/results/derived-result-visibility';
import { useRecordingActivity } from '@/features/ai/use-recording-activity';
import { useRecordingAiProcessingState } from '@/features/ai/use-recording-ai-processing-state';
import { useRecordingVisualObservations } from '@/features/ai/use-recording-visual-observations';
import { useStudioResultRange } from '../components/studio/results/studio-result-window-store';
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
  const redactionSignature = useMemo(
    () => recordingRedactionSignature(detail),
    [detail],
  );
  const resultRange = useStudioResultRange(recordingId);
  const aiProcessing = useRecordingAiProcessingState(
    recordingId,
    redactionSignature,
    resultRange,
  );
  const activity = useRecordingActivity(
    recordingId,
    aiProcessing.state,
    redactionSignature,
    resultRange,
  );
  const visualObservations = useRecordingVisualObservations(
    recordingId,
    aiProcessing.state,
    redactionSignature,
    resultRange,
  );
  const ocrTimeline = useOcrTimelineFrames(
    recordingId,
    redactionSignature,
    aiProcessing.state,
    resultRange,
  );
  const visibleOcrFrames = useMemo(
    () => (detail ? visibleOcrTimelineFrames(detail, ocrTimeline.frames) : []),
    [detail, ocrTimeline.frames],
  );
  const visibleActivityState = useMemo(
    () => (detail ? visibleActivity(detail, activity.state) : null),
    [activity.state, detail],
  );
  const visibleAiState = useMemo(
    () =>
      detail ? visibleAiProcessingState(detail, aiProcessing.state) : null,
    [aiProcessing.state, detail],
  );
  const visibleVisualState = useMemo(
    () =>
      detail
        ? visibleVisualObservations(detail, visualObservations.state)
        : null,
    [detail, visualObservations.state],
  );
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
      const nextDetail = await getLibraryRecording(recordingId, {
        includeDiskUsage: false,
      });
      if (!isCurrentLoad()) {
        return;
      }
      setDetail(nextDetail);
      setStatus('ready');
      if (nextDetail.availability === 'available') {
        createLibraryPlaybackSession({ recordingId, lazyThumbnails: true })
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
      throw deleteError;
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
      const job = aiProcessing.state?.job;
      if (job?.status === 'failed' || job?.status === 'completed-with-errors')
        await retryProcessingJob({ jobId: job.id });
      else if (job?.status === 'paused')
        await resumeProcessingJob({ jobId: job.id });
      else await startRecordingAiProcessing({ recordingId: detail.id });
      await aiProcessing.reload();
    } catch (processingError) {
      setError(errorMessage(processingError));
    } finally {
      setProcessingStartBusy(false);
    }
  }, [aiProcessing, detail]);

  const refresh = useCallback(async () => {
    await Promise.all([
      load(),
      aiProcessing.reload(),
      activity.reload(),
      visualObservations.reload(),
      ocrTimeline.reload(),
    ]);
  }, [
    activity.reload,
    aiProcessing.reload,
    load,
    ocrTimeline.reload,
    visualObservations.reload,
  ]);

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
      load: refresh,
      removeRedaction,
      startAiProcessing,
      unredactRanges,
    },
    state: {
      activity: { ...activity, state: visibleActivityState },
      aiProcessing: { ...aiProcessing, state: visibleAiState },
      detail,
      error:
        error ??
        aiProcessing.error ??
        activity.error ??
        visualObservations.error,
      ocrTimelineError: ocrTimeline.error,
      ocrFrames: visibleOcrFrames,
      ocrTimelineStatus: ocrTimeline.status,
      playbackAssets,
      processingStartBusy,
      status,
      thumbnailStatus,
      visualObservations: { ...visualObservations, state: visibleVisualState },
    },
  };
}

function recordingRedactionSignature(
  detail: LibraryRecordingDetail | null,
): string {
  return (
    detail?.redactions
      .map((redaction) =>
        [
          redaction.id,
          redaction.startOffsetMs,
          redaction.endOffsetMs,
          [...redaction.trackIds].sort().join(','),
        ].join(':'),
      )
      .sort()
      .join('|') ?? ''
  );
}
