import { useCallback, useEffect, useRef, useState } from 'react';
import { listenToRecallStateChanged } from './recall-client';

const metadataQuietMs = 750;

export type RecallRead<T> =
  | { status: 'loading' | 'invalidated'; data: null }
  | { status: 'ready'; data: T }
  | { status: 'error'; data: null };

/** Private reads are discarded synchronously when source eligibility changes. */
export function useRecallRead<T extends { privacyRevision: number }>(
  read: () => Promise<T>,
  cancelRead?: () => Promise<void>,
  readInitialPrivacyRevision?: () => Promise<number>,
) {
  const [cache, setCache] = useState<{
    owner: () => Promise<T>;
    state: RecallRead<T>;
    isRefreshing: boolean;
    checkedAt: number | null;
  }>({
    owner: read,
    state: { status: 'loading', data: null },
    isRefreshing: true,
    checkedAt: null,
  });
  const [revision, setRevision] = useState(0);
  const ownership = useRef(0);
  const privacyRevision = useRef<number | null>(null);
  const cancellation = useRef<Promise<void>>(Promise.resolve());
  const refresh = useCallback(() => {
    ownership.current += 1;
    setCache((current) => ({
      ...current,
      isRefreshing: true,
      state:
        current.state.status === 'ready'
          ? current.state
          : { status: 'loading', data: null },
    }));
    setRevision((value) => value + 1);
  }, []);

  useEffect(() => {
    let active = true;
    let unsubscribe: (() => void) | undefined;
    let readStarted = false;
    let readInFlight = true;
    let cancellationIssued = false;
    let pendingRefresh = false;
    let quietPassed = false;
    let quietTimer: ReturnType<typeof setTimeout> | undefined;
    const request = ++ownership.current;
    setCache((current) =>
      current.owner === read
        ? current.isRefreshing
          ? current
          : { ...current, isRefreshing: true }
        : {
            owner: read,
            state: { status: 'loading', data: null },
            isRefreshing: true,
            checkedAt: null,
          },
    );
    function cancelActiveRead() {
      if (readStarted && readInFlight && cancelRead && !cancellationIssued) {
        cancellationIssued = true;
        cancellation.current = cancelRead().catch(() => undefined);
      }
    }
    function queueMetadataRefresh() {
      pendingRefresh = true;
      quietPassed = false;
      clearTimeout(quietTimer);
      quietTimer = setTimeout(() => {
        quietPassed = true;
        if (active && !readInFlight) setRevision((value) => value + 1);
      }, metadataQuietMs);
    }
    async function load() {
      try {
        // Subscribe before reading so invalidation cannot slip between them.
        const cleanup = await listenToRecallStateChanged(
          (event) => {
            if (!active) return;
            if (privacyRevision.current === null && !readStarted) {
              privacyRevision.current = event.privacyRevision;
              queueMetadataRefresh();
              return;
            }
            if (event.privacyRevision !== privacyRevision.current) {
              ownership.current += 1;
              clearTimeout(quietTimer);
              pendingRefresh = false;
              cancelActiveRead();
              setCache({
                owner: read,
                state: { status: 'invalidated', data: null },
                isRefreshing: true,
                checkedAt: null,
              });
              privacyRevision.current = event.privacyRevision;
              setRevision((value) => value + 1);
            } else queueMetadataRefresh();
          },
          () => {
            if (!active) return;
            active = false;
            ownership.current += 1;
            privacyRevision.current = null;
            clearTimeout(quietTimer);
            unsubscribe?.();
            cancelActiveRead();
            setCache({
              owner: read,
              state: { status: 'error', data: null },
              isRefreshing: false,
              checkedAt: null,
            });
          },
        );
        if (!active) {
          cleanup();
          return;
        }
        unsubscribe = cleanup;
        await cancellation.current;
        if (!active) return;
        if (privacyRevision.current === null && readInitialPrivacyRevision) {
          const initialRevision = await readInitialPrivacyRevision();
          if (!active) return;
          if (privacyRevision.current === null)
            privacyRevision.current = initialRevision;
        }
        readStarted = true;
        const data = await read();
        if (active && request === ownership.current) {
          if (
            privacyRevision.current !== null &&
            data.privacyRevision < privacyRevision.current
          ) {
            setCache({
              owner: read,
              state: { status: 'invalidated', data: null },
              isRefreshing: false,
              checkedAt: null,
            });
            return;
          }
          privacyRevision.current = data.privacyRevision;
          setCache({
            owner: read,
            state: { status: 'ready', data },
            isRefreshing: false,
            checkedAt: Date.now(),
          });
        }
      } catch {
        if (active && request === ownership.current)
          setCache({
            owner: read,
            state: { status: 'error', data: null },
            isRefreshing: false,
            checkedAt: null,
          });
      } finally {
        readInFlight = false;
        if (active && pendingRefresh && quietPassed)
          setRevision((value) => value + 1);
      }
    }
    void load();
    return () => {
      active = false;
      ownership.current += 1;
      unsubscribe?.();
      clearTimeout(quietTimer);
      cancelActiveRead();
    };
  }, [read, revision, cancelRead, readInitialPrivacyRevision]);
  return {
    isRefreshing: cache.owner === read ? cache.isRefreshing : true,
    checkedAt: cache.owner === read ? cache.checkedAt : null,
    state:
      cache.owner === read
        ? cache.state
        : ({ status: 'loading', data: null } satisfies RecallRead<T>),
    refresh,
  };
}
