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
  range?: { startOffsetMs: number; endOffsetMs: number; bucketMs?: number },
) {
  const key = JSON.stringify([recordingId, privacyRevision]);
  const revision = JSON.stringify([
    processingState?.chunks
      .filter((chunk) => chunk.sourceKind === 'ocr')
      .map((chunk) => [chunk.id, chunk.status, chunk.updatedAtMs]),
    processingState?.visualFrames
      ?.filter((frame) => frame.sourceKind === 'ocr')
      .map((frame) => [frame.id, frame.status, frame.updatedAtMs]),
    processingState?.reprocessingScopes,
  ]);
  const requestRef = useRef(0);
  const [result, setResult] = useState<{
    key: string;
    frames: OcrTimelineFrame[];
  } | null>(null);
  const [status, setStatus] = useState<'loading' | 'ready' | 'error'>(
    'loading',
  );
  const [error, setError] = useState<string | null>(null);
  const reload = useCallback(async () => {
    const request = ++requestRef.current;
    if (range?.bucketMs) {
      setResult({ key, frames: [] });
      setStatus('ready');
      return;
    }
    setStatus('loading');
    setError(null);
    setResult((current) => (current?.key === key ? current : null));
    let cursor: OcrHistoryCursor | null = null;
    const frames = new Map<string, OcrTimelineFrame>();
    const seenCursors = new Set<string>();
    try {
      do {
        const page = await getOcrTimelineFrames({
          recordingId,
          cursor,
          limit: 1_000,
          ...(range
            ? {
                startTimestampMs: range.startOffsetMs,
                endTimestampMs: range.endOffsetMs,
              }
            : {}),
        });
        if (request !== requestRef.current) return;
        for (const frame of page.frames) frames.set(frame.id, frame);
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
      setResult({ key, frames: [...frames.values()] });
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
  }, [key, recordingId, range]);
  useEffect(() => {
    void reload();
    return () => {
      requestRef.current += 1;
    };
  }, [reload, revision]);
  return {
    frames: result?.key === key ? result.frames : [],
    error,
    reload,
    status,
  };
}
