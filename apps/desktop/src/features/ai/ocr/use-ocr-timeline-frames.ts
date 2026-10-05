import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  OcrTimelineFrame,
  OcrHistoryCursor,
  RecordingAiProcessingState,
} from '@daydream/shared';
import { getOcrTimelineFrames } from './ocr-client';

export function useOcrTimelineFrames(
  recordingId: string,
  privacyRevision: string,
  processingState: RecordingAiProcessingState | null,
) {
  const key = JSON.stringify([recordingId, privacyRevision]);
  const requestRef = useRef(0);
  const [result, setResult] = useState<{
    key: string;
    processingState: RecordingAiProcessingState | null;
    frames: OcrTimelineFrame[];
  } | null>(null);
  const [status, setStatus] = useState<'loading' | 'ready' | 'error'>(
    'loading',
  );
  const [error, setError] = useState<string | null>(null);
  const reload = useCallback(async () => {
    const request = ++requestRef.current;
    setStatus('loading');
    setError(null);
    setResult((current) =>
      current?.key === key && current.processingState === processingState
        ? current
        : null,
    );
    let cursor: OcrHistoryCursor | null = null;
    const frames = new Map<string, OcrTimelineFrame>();
    const seenCursors = new Set<string>();
    try {
      do {
        const page = await getOcrTimelineFrames({
          recordingId,
          cursor,
          limit: 1_000,
        });
        if (request !== requestRef.current) return;
        for (const frame of page.frames) frames.set(frame.id, frame);
        setResult({ key, processingState, frames: [...frames.values()] });
        cursor = page.nextCursor;
        if (cursor !== null) {
          const cursorKey = JSON.stringify(cursor);
          if (seenCursors.has(cursorKey))
            throw new Error(
              'The OCR observation index returned a repeated page cursor.',
            );
          seenCursors.add(cursorKey);
        }
      } while (cursor !== null);
      setStatus('ready');
    } catch (failure) {
      if (request !== requestRef.current) return;
      setError(
        failure instanceof Error
          ? failure.message
          : 'OCR observations could not be loaded.',
      );
      setStatus('error');
    }
  }, [key, processingState, recordingId]);
  useEffect(() => {
    void reload();
    return () => {
      requestRef.current += 1;
    };
  }, [reload]);
  return {
    frames:
      result?.key === key && result.processingState === processingState
        ? result.frames
        : [],
    error,
    reload,
    status,
  };
}
