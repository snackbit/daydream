import { useCallback, useEffect, useRef, useState } from 'react';
import type { OcrHistoryEntry, OcrHistoryPage } from '@daydream/shared';
import { getOcrHistory } from './ocr-client';

export function useOcrHistory({
  recordingId,
  sourceTrackId,
  privacyRevision,
  processingRevision = '',
  enabled,
}: {
  recordingId: string;
  sourceTrackId: string;
  privacyRevision: string;
  processingRevision?: string;
  enabled: boolean;
}) {
  const key = JSON.stringify([recordingId, sourceTrackId, privacyRevision]);
  const run = useRef(0);
  const [result, setResult] = useState<{
    key: string;
    items: OcrHistoryEntry[];
    cursor: OcrHistoryPage['nextCursor'];
  } | null>(null);
  const [status, setStatus] = useState<'idle' | 'loading' | 'ready' | 'error'>(
    'idle',
  );
  const [error, setError] = useState<string | null>(null);
  const visible = result?.key === key ? result : null;
  const cursor = visible?.cursor ?? null;
  const load = useCallback(
    async (
      append: boolean,
      nextCursor: OcrHistoryPage['nextCursor'] = null,
    ) => {
      const request = ++run.current;
      setStatus('loading');
      setError(null);
      try {
        const page = await getOcrHistory({
          recordingId,
          sourceTrackId,
          cursor: nextCursor,
          limit: 20,
        });
        if (request !== run.current) return;
        setResult((current) => ({
          // Native history IDs identify occurrences, not reusable image content.
          // Nonadjacent identical images therefore remain distinct revision rows.
          key,
          items:
            append && current?.key === key
              ? [
                  ...current.items,
                  ...page.items.filter(
                    (item) =>
                      !current.items.some(
                        (previous) => previous.id === item.id,
                      ),
                  ),
                ]
              : page.items,
          cursor: page.nextCursor,
        }));
        setStatus('ready');
      } catch (failure) {
        if (request !== run.current) return;
        setError(
          failure instanceof Error
            ? failure.message
            : 'OCR history could not be loaded.',
        );
        setStatus('error');
      }
    },
    [key, recordingId, sourceTrackId],
  );
  useEffect(() => {
    if (enabled) void load(false);
    return () => {
      run.current += 1;
    };
  }, [enabled, load, processingRevision]);
  const loadMore = useCallback(() => load(true, cursor), [cursor, load]);
  const retry = useCallback(() => load(false), [load]);
  return {
    error,
    items: visible?.items ?? [],
    loadMore,
    retry,
    hasMore: cursor !== null,
    status: visible === null && status === 'ready' ? 'loading' : status,
  };
}
