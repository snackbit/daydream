import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  CreateLibraryRedactionRangeInput,
  ResizeLibraryRedactionInput,
} from '@daydream/shared';
import {
  createLibraryRedactions,
  deleteLibraryRedaction,
  resizeLibraryRedaction,
  unredactLibraryRedactions,
} from '../library-client';

export type DayPrivacyOperation =
  | {
      kind: 'ranges';
      recordingId: string;
      ranges: CreateLibraryRedactionRangeInput[];
      hide: boolean;
    }
  | { kind: 'delete'; recordingId: string; redactionId: string }
  | { kind: 'resize'; input: ResizeLibraryRedactionInput };
/** Retain retries above privacy-driven Studio remounts, with source-local inputs. */
export function useDayMutations(dayId: string, onRefresh: () => void) {
  const [state, setState] = useState<{
    dayId: string;
    pending: boolean;
    failed: DayPrivacyOperation[];
  }>({ dayId, pending: false, failed: [] });
  const generation = useRef(0),
    pending = useRef(false);
  useEffect(() => {
    generation.current++;
    return () => {
      generation.current++;
    };
  }, [dayId]);
  const run = useCallback(
    async (operations: DayPrivacyOperation[]) => {
      if (pending.current)
        throw new Error('A privacy change is already being saved.');
      const request = generation.current;
      pending.current = true;
      setState({ dayId, pending: true, failed: [] });
      const failed: DayPrivacyOperation[] = [];
      try {
        for (const operation of operations) {
          try {
            if (operation.kind === 'ranges')
              await (operation.hide
                ? createLibraryRedactions({
                    recordingId: operation.recordingId,
                    ranges: operation.ranges,
                  })
                : unredactLibraryRedactions({
                    recordingId: operation.recordingId,
                    ranges: operation.ranges,
                  }));
            else if (operation.kind === 'delete')
              await deleteLibraryRedaction({
                recordingId: operation.recordingId,
                redactionId: operation.redactionId,
              });
            else await resizeLibraryRedaction(operation.input);
          } catch {
            failed.push(operation);
          }
        }
      } finally {
        pending.current = false;
        if (request === generation.current) {
          setState({ dayId, pending: false, failed });
          onRefresh();
        }
      }
      if (failed.length)
        throw new Error(
          'Some privacy changes could not be saved. Retry the remaining changes.',
        );
    },
    [dayId, onRefresh],
  );
  const retry = useCallback(() => {
    if (state.dayId === dayId) void run(state.failed).catch(() => undefined);
  }, [run, state, dayId]);
  return {
    run,
    retry,
    pending: state.dayId === dayId && state.pending,
    failedCount: state.dayId === dayId ? state.failed.length : 0,
  };
}
export type DayMutationController = ReturnType<typeof useDayMutations>;
